/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! x86 virtual memory works with a tree structure. Each element is an array of
//! sub-elements. The position of the elements in the arrays allows to tell the
//! virtual address for the mapping.
//!
//! Under 32 bits, elements are array of 32 bits long words that can contain 1024 entries.
//!
//! The following elements are available:
//! - Page directory: The main element, contains page tables
//! - Page table: Represents a block of 4MB, each entry is a page
//!
//! Under 32 bits, pages are 4096 bytes large.
//!
//! Each entry of elements contains the physical address to the element/page and some flags.
//! The flags can be stored with the address in only 4 bytes large entries because addresses have
//! to be page-aligned, freeing 12 bits in the entry for the flags.
//!
//! For each entry of each element, the kernel must keep track of how many
//! elements are being used. This can be done with a simple counter: when an
//! entry is allocated, the counter is incremented and when an entry is freed,
//! the counter is decremented. When the counter reaches 0, the element can be
//! freed.
//!
//! The Page Size Extension (PSE) allows to map 4MB large blocks without using a
//! page table.

use crate::{
	arch::x86::supports_supervisor_prot,
	memory::{buddy, PhysAddr, VirtAddr},
	register_get, register_set,
};
use core::{
	arch::asm,
	mem,
	ops::{Deref, DerefMut},
	ptr::{addr_of, NonNull},
};
use utils::{collections::vec::Vec, errno::AllocResult, limits::PAGE_SIZE};

/// Paging entry.
#[cfg(target_arch = "x86")]
pub type Entry = u32;
/// Paging entry.
#[cfg(target_arch = "x86_64")]
pub type Entry = u64;

#[cfg(target_arch = "x86_64")]
/// **x86 paging flag**: If set, execution of instruction is disabled.
pub const FLAG_XD: Entry = 1 << 63;
/// **x86 paging flag**: If set, prevents the CPU from updating the associated
/// addresses when the TLB is flushed.
pub const FLAG_GLOBAL: Entry = 0b100000000;
/// **x86 paging flag**: If set, pages are 4 MB long.
pub const FLAG_PAGE_SIZE: Entry = 0b010000000;
/// **x86 paging flag**: Indicates that the page has been written.
pub const FLAG_DIRTY: Entry = 0b001000000;
/// **x86 paging flag**: Set if the page has been read or written.
pub const FLAG_ACCESSED: Entry = 0b000100000;
/// **x86 paging flag**: If set, page will not be cached.
pub const FLAG_CACHE_DISABLE: Entry = 0b000010000;
/// **x86 paging flag**: If set, write-through caching is enabled.
/// If not, then write-back is enabled instead.
pub const FLAG_WRITE_THROUGH: Entry = 0b000001000;
/// **x86 paging flag**: If set, the page can be accessed by userspace operations.
pub const FLAG_USER: Entry = 0b000000100;
/// **x86 paging flag**: If set, the page can be written.
pub const FLAG_WRITE: Entry = 0b000000010;
/// **x86 paging flag**: If set, the page is present.
pub const FLAG_PRESENT: Entry = 0b000000001;

/// Flags mask in a page directory entry.
pub const FLAGS_MASK: Entry = 0xfff;
/// Address mask in a page directory entry. The address doesn't need every byte
/// since it must be page-aligned.
pub const ADDR_MASK: Entry = !FLAGS_MASK;

/// x86 page fault flag. If set, the page was present.
pub const PAGE_FAULT_PRESENT: u32 = 0b00001;
/// x86 page fault flag. If set, the error was caused by a write operation, else
/// the error was caused by a read operation.
pub const PAGE_FAULT_WRITE: u32 = 0b00010;
/// x86 page fault flag. If set, the page fault was caused by an userspace
/// operation.
pub const PAGE_FAULT_USER: u32 = 0b00100;
/// x86 page fault flag. If set, one or more page directory entries contain
/// reserved bits which are set.
pub const PAGE_FAULT_RESERVED: u32 = 0b01000;
/// x86 page fault flag. If set, the page fault was caused by an instruction
/// fetch.
pub const PAGE_FAULT_INSTRUCTION: u32 = 0b10000;

/// The number of entries in a table.
pub const ENTRIES_PER_TABLE: usize = if cfg!(target_arch = "x86") { 1024 } else { 512 };
/// The paging level.
#[cfg(target_arch = "x86")]
pub const DEPTH: usize = 2;
/// The paging level.
#[cfg(target_arch = "x86_64")]
pub const DEPTH: usize = 4;

/// The number of tables reserved for the userspace.
///
/// Those tables start at the beginning of the page directory. Remaining tables are reserved for
/// the kernel.
const USERSPACE_TABLES: usize = if cfg!(target_arch = "x86") { 768 } else { 256 };
/// The number of tables reserved for the kernelspace.
const KERNELSPACE_TABLES: usize = ENTRIES_PER_TABLE - USERSPACE_TABLES;
/// Kernel space entries flags.
const KERNEL_FLAGS: Entry = FLAG_PRESENT | FLAG_WRITE | FLAG_USER | FLAG_GLOBAL;

/// Paging table.
#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct Table(pub [Entry; ENTRIES_PER_TABLE]);

impl Deref for Table {
	type Target = [Entry; ENTRIES_PER_TABLE];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Table {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl Table {
	/// Expands the PSE entry at `index` into a new table.
	///
	/// This function allocates a new page table and fills it so that the memory mapping keeps the
	/// same behavior.
	pub fn expand(&mut self, index: usize) -> AllocResult<()> {
		let entry = self[index];
		if entry & FLAG_PRESENT == 0 || entry & FLAG_PAGE_SIZE == 0 {
			return Ok(());
		}
		let addr = entry & ADDR_MASK;
		let flags = entry & (FLAGS_MASK & !FLAG_PAGE_SIZE);
		// Create table
		let mut new_table = alloc_table()?;
		let new_table_ref = unsafe { new_table.as_mut() };
		new_table_ref.iter_mut().enumerate().for_each(|(i, e)| {
			// FIXME the stride can be more than PAGE_SIZE depending on whether we are on 32 or 64
			// bit and the level of the paging object
			let addr = VirtAddr(addr as usize) + i * PAGE_SIZE;
			let addr = addr.kernel_to_physical().unwrap();
			*e = to_entry(addr, flags);
		});
		// Set new entry
		let addr = VirtAddr::from(new_table).kernel_to_physical().unwrap();
		self[index] = to_entry(addr, flags);
		Ok(())
	}

	/// Tells whether the table at index `index` in the page directory is empty.
	pub fn is_empty(&self) -> bool {
		// TODO Use a counter instead. Increment it when mapping a page in the table and
		// decrement it when unmapping. Then return `true` if the counter has the value
		// `0`
		self.iter().all(|e| e & FLAG_PRESENT == 0)
	}
}

/// Kernel space paging tables common to every context.
static mut KERNEL_TABLES: [Table; KERNELSPACE_TABLES] =
	[Table([0; ENTRIES_PER_TABLE]); KERNELSPACE_TABLES];

/// Allocates a table and returns its virtual address.
///
/// If the allocation fails, the function returns an error.
fn alloc_table() -> AllocResult<NonNull<Table>> {
	let mut table = buddy::alloc_kernel(0)?.cast::<Table>();
	unsafe {
		table.as_mut().fill(0);
	}
	Ok(table)
}

/// Frees a table.
///
/// # Safety
///
/// Further accesses to the table after this function are undefined.
unsafe fn free_table(table: NonNull<Table>) {
	buddy::free_kernel(table.as_ptr() as _, 0);
}

/// Turns the given object/flags pair into an entry for another object.
///
/// Invalid flags are ignored and the [`FLAG_PRESENT`] flag is inserted automatically.
#[inline]
fn to_entry(addr: PhysAddr, flags: Entry) -> Entry {
	// Sanitize flags
	let flags = flags & FLAGS_MASK | FLAG_PRESENT;
	// Address alignment guarantees the address does not overlap flags
	addr.0 as Entry | flags
}

/// Turns an entry back into an object/flags pair.
///
/// # Safety
///
/// If the object's address in the entry is invalid, the behaviour is undefined.
#[inline]
unsafe fn unwrap_entry(entry: Entry) -> (NonNull<Table>, Entry) {
	let table = PhysAddr((entry & ADDR_MASK) as usize)
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	let table = NonNull::new(table).unwrap();
	let flags = entry & FLAGS_MASK;
	(table, flags)
}

/// Allocates and initializes a virtual memory context.
///
/// The kernel memory is mapped into the context by default.
pub fn alloc() -> AllocResult<NonNull<Table>> {
	let mut ctx = alloc_table()?;
	// Init kernel entries
	let kernel_tables = addr_of!(KERNEL_TABLES) as *const Table;
	let ctx_ref = unsafe { ctx.as_mut() };
	ctx_ref[USERSPACE_TABLES..]
		.iter_mut()
		.enumerate()
		.for_each(|(i, dst)| {
			let addr = unsafe { kernel_tables.add(i) };
			let addr = VirtAddr::from(addr).kernel_to_physical().unwrap();
			*dst = to_entry(addr, KERNEL_FLAGS);
		});
	Ok(ctx)
}

/// Returns the index of the element corresponding to the given virtual
/// address `addr` for element at level `level` in the tree.
///
/// The level represents the depth in the tree. `0` is the deepest.
#[inline]
fn get_addr_element_index(addr: VirtAddr, level: usize) -> usize {
	#[cfg(target_arch = "x86")]
	{
		(addr.0 >> (12 + level * 10)) & 0x3ff
	}
	#[cfg(target_arch = "x86_64")]
	{
		(addr.0 >> (12 + level * 9)) & 0x1ff
	}
}

/// Returns the corresponding entry for [`translate`].
fn translate_impl(mut table: &Table, addr: VirtAddr) -> Option<Entry> {
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(addr, level);
		let entry = table[index];
		if entry & FLAG_PRESENT == 0 {
			break;
		}
		if level == 0 {
			return Some(entry);
		}
		if entry & FLAG_PAGE_SIZE != 0 {
			return Some(entry);
		}
		// Jump to next table
		let phys_addr = PhysAddr((entry & ADDR_MASK) as _);
		let virt_addr = phys_addr.kernel_to_virtual().unwrap();
		table = unsafe { &*virt_addr.as_ptr() };
	}
	None
}

/// Translates the given virtual address `addr` to the corresponding physical address using
/// `page_dir`.
pub fn translate(page_dir: &Table, addr: VirtAddr) -> Option<PhysAddr> {
	let entry = translate_impl(page_dir, addr)?;
	let remain_mask = if entry & FLAG_PAGE_SIZE == 0 {
		PAGE_SIZE - 1
	} else {
		ENTRIES_PER_TABLE * PAGE_SIZE - 1
	};
	let mut phys_addr = (entry & ADDR_MASK) as usize;
	phys_addr |= addr.0 & remain_mask;
	Some(PhysAddr(phys_addr))
}

/// Memory paging rollback hook, allowing to undo modifications on a virtual memory context if an
/// operation in a transaction fails, allowing to preserve integrity.
pub struct Rollback {
	/// The list of modified entries, with their respective previous value and a boolean
	/// indicating whether the underlying table could be freed.
	///
	/// This field works in a FIFO fashion. That is, the rollback operation must begin with the
	/// last element.
	entries: Vec<(NonNull<Entry>, Entry, bool)>,
}

impl Rollback {
	fn cleanup_table(cur_entry: Entry, prev_entry: Entry) {
		let cur_has_table = cur_entry & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT;
		let prev_has_table = prev_entry & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT;
		if !cur_has_table && prev_has_table {
			unsafe {
				let (mut table_ptr, _) = unwrap_entry(prev_entry);
				let table = table_ptr.as_mut();
				if table.is_empty() {
					free_table(table_ptr);
				}
			}
		}
	}

	/// Rollbacks the operation on the given `table`.
	#[cold]
	pub fn rollback(mut self) {
		let entries = mem::take(&mut self.entries);
		for (mut ptr, prev_entry, free) in entries.into_iter().rev() {
			let ent = unsafe { ptr.as_mut() };
			// Reverse entry change
			let prev_entry = mem::replace(ent, prev_entry);
			if free {
				// If a table was just created by the operation and is now empty, remove it
				Self::cleanup_table(*ent, prev_entry)
			}
		}
	}
}

impl Drop for Rollback {
	fn drop(&mut self) {
		// Remove old tables if empty
		let entries = mem::take(&mut self.entries);
		for (mut ptr, prev_entry, free) in entries.into_iter().rev() {
			let ent = unsafe { ptr.as_mut() };
			if free {
				// If a table was just removed by the operation and is now empty, remove it
				Self::cleanup_table(*ent, prev_entry)
			}
		}
	}
}

/// Inner implementation of [`crate::memory::vmem::VMemTransaction::map`] for x86.
///
/// # Safety
///
/// In case the mapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub unsafe fn map(
	mut table: &mut Table,
	physaddr: PhysAddr,
	virtaddr: VirtAddr,
	flags: Entry,
) -> AllocResult<Rollback> {
	// Sanitize
	let physaddr = PhysAddr(physaddr.0 & !(PAGE_SIZE - 1));
	let virtaddr = VirtAddr(virtaddr.0 & !(PAGE_SIZE - 1));
	// TODO support FLAG_PAGE_SIZE (requires a way to specify a which level it must be enabled)
	let flags = (flags & FLAGS_MASK & !FLAG_PAGE_SIZE) | FLAG_PRESENT;
	// Set entries
	let mut previous_entries = Vec::with_capacity(DEPTH)?;
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(virtaddr, level);
		let previous_entry = table[index];
		// Add entry for rollback
		let may_remove_table = level > 0 && (level < DEPTH - 1 || index < USERSPACE_TABLES);
		previous_entries
			.push((
				NonNull::from(&table[index]),
				previous_entry,
				may_remove_table,
			))
			.unwrap();
		if level == 0 {
			table[index] = to_entry(physaddr, flags);
			break;
		}
		// Allocate a table if necessary
		if previous_entry & FLAG_PRESENT == 0 {
			// No table is present, allocate one
			let new_table = alloc_table()?;
			let addr = VirtAddr::from(new_table).kernel_to_physical().unwrap();
			table[index] = to_entry(addr, flags);
		} else if previous_entry & FLAG_PAGE_SIZE != 0 {
			// A PSE entry is present, need to expand it for the mapping
			table.expand(index)?;
		}
		table[index] |= flags;
		// Jump to next table
		table = unsafe { unwrap_entry(table[index]).0.as_mut() };
	}
	Ok(Rollback {
		entries: previous_entries,
	})
}

/// Inner implementation of [`crate::memory::vmem::VMemTransaction::unmap`] for x86.
///
/// # Safety
///
/// In case the unmapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub unsafe fn unmap(mut table: &mut Table, virtaddr: VirtAddr) -> AllocResult<Rollback> {
	// Sanitize
	let virtaddr = VirtAddr(virtaddr.0 & !(PAGE_SIZE - 1));
	// Set entries
	let mut previous_entries = Vec::with_capacity(DEPTH)?;
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(virtaddr, level);
		// Remove entry and get previous value
		let previous_entry = mem::replace(&mut table[index], 0);
		// Add entry for rollback
		let may_remove_table = level > 0 && (level < DEPTH - 1 || index < USERSPACE_TABLES);
		previous_entries
			.push((
				NonNull::from(&table[index]),
				previous_entry,
				may_remove_table,
			))
			.unwrap();
		// If the entry did not exist or was PSE, stop here
		if previous_entry & FLAG_PRESENT == 0 || previous_entry & FLAG_PAGE_SIZE != 0 {
			break;
		}
		// Jump to next table
		table = unsafe { unwrap_entry(previous_entry).0.as_mut() };
	}
	Ok(Rollback {
		entries: previous_entries,
	})
}

/// Binds the given page directory to the current CPU.
///
/// # Safety
///
/// The caller must ensure the given page directory is correct.
/// Meaning it must be mapping the kernel's code and data sections, and any regions of memory that
/// might be accessed in the future.
#[inline]
pub unsafe fn bind(page_dir: PhysAddr) {
	asm!(
		"mov cr3, {dir}",
		dir = in(reg) page_dir.0
	)
}

/// Tells whether the given page directory is bound on the current CPU.
#[inline]
pub fn is_bound(page_dir: NonNull<Table>) -> bool {
	let physaddr = VirtAddr::from(page_dir).kernel_to_physical().unwrap();
	register_get!("cr3") == physaddr.0
}

/// Invalidate the page from the TLB at the given address on the current CPU.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn invlpg(addr: VirtAddr) {
	unsafe {
		asm!("invlpg [{addr}]", addr = in(reg) addr.0, options(nostack));
	}
}

/// Flush the Translation Lookaside Buffer (TLB) on the current CPU.
pub fn flush_current() {
	unsafe {
		asm!(
			"mov {tmp}, cr3",
			"mov cr3, {tmp}",
			tmp = out(reg) _
		);
	}
}

/// Destroys the given page directory, including its children elements.
///
/// # Safety
///
/// It is assumed the context is not being used.
///
/// Subsequent uses of `page_dir` are undefined.
pub unsafe fn free(mut page_dir: NonNull<Table>) {
	let pd = unsafe { page_dir.as_mut() };
	for entry in &pd[..USERSPACE_TABLES] {
		let (table, flags) = unwrap_entry(*entry);
		if flags & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT {
			free_table(table);
		}
	}
	free_table(page_dir);
}

/// Prepares for virtual memory management on the current CPU.
pub(crate) fn prepare() {
	// Set cr4 flags
	// Enable GLOBAL flag
	let mut cr4 = register_get!("cr4") | 1 << 7;
	let (smep, smap) = supports_supervisor_prot();
	if smep {
		cr4 |= 1 << 20;
	}
	if smap {
		cr4 |= 1 << 21;
	}
	unsafe {
		register_set!("cr4", cr4);
	}
}
