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

//! x86 virtual memory support.

use crate::{
	arch::x86::supports_supervisor_prot,
	memory::{buddy, buddy::BUDDY_RETRY, PhysAddr, VirtAddr},
	register_get, register_set,
};
use core::{
	arch::asm,
	mem,
	ops::{Deref, DerefMut},
	ptr::NonNull,
	sync::atomic::{AtomicUsize, Ordering::Relaxed},
};
use utils::limits::PAGE_SIZE;

/// Paging entry.
type Entry = AtomicUsize;

/// **x86 paging flag**: If set, execution of instruction is disabled.
#[cfg(target_arch = "x86_64")]
pub const FLAG_XD: usize = 1 << 63;
/// **x86 paging flag**: If set, prevents the CPU from updating the associated
/// addresses when the TLB is flushed.
pub const FLAG_GLOBAL: usize = 0b100000000;
/// **x86 paging flag**: If set, pages are 4 MB long.
pub const FLAG_PAGE_SIZE: usize = 0b010000000;
/// **x86 paging flag**: Indicates that the page has been written.
pub const FLAG_DIRTY: usize = 0b001000000;
/// **x86 paging flag**: Set if the page has been read or written.
pub const FLAG_ACCESSED: usize = 0b000100000;
/// **x86 paging flag**: If set, page will not be cached.
pub const FLAG_CACHE_DISABLE: usize = 0b000010000;
/// **x86 paging flag**: If set, write-through caching is enabled.
/// If not, then write-back is enabled instead.
pub const FLAG_WRITE_THROUGH: usize = 0b000001000;
/// **x86 paging flag**: If set, the page can be accessed by userspace operations.
pub const FLAG_USER: usize = 0b000000100;
/// **x86 paging flag**: If set, the page can be written.
pub const FLAG_WRITE: usize = 0b000000010;
/// **x86 paging flag**: If set, the page is present.
pub const FLAG_PRESENT: usize = 0b000000001;

/// Flags mask in a page directory entry.
pub const FLAGS_MASK: usize = ((1u64 << 63) | 0xfff) as usize;
/// Address mask in a page directory entry. The address doesn't need every byte
/// since it must be page-aligned.
pub const ADDR_MASK: usize = !FLAGS_MASK;

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
const KERNEL_FLAGS: usize = FLAG_PRESENT | FLAG_WRITE | FLAG_GLOBAL;

/// Paging table.
#[repr(C, align(4096))]
pub struct Table(pub [Entry; ENTRIES_PER_TABLE]);

impl Table {
	/// Creates a new zeroed table.
	pub const fn new() -> Self {
		Self(unsafe { mem::zeroed() })
	}
}

impl Default for Table {
	fn default() -> Self {
		Self::new()
	}
}

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
	pub fn expand(&mut self, index: usize) {
		let entry = self[index].load(Relaxed);
		if entry & FLAG_PRESENT == 0 || entry & FLAG_PAGE_SIZE == 0 {
			return;
		}
		let flags = entry & (FLAGS_MASK & !FLAG_PAGE_SIZE);
		// Create table
		let mut new_table = alloc_table();
		let new_table_ref = unsafe { new_table.as_mut() };
		new_table_ref.iter_mut().enumerate().for_each(|(i, e)| {
			// FIXME the stride can be more than PAGE_SIZE depending on whether we are on 32 or 64
			// bit and the level of the paging object
			let addr = VirtAddr(entry & ADDR_MASK) + i * PAGE_SIZE;
			let addr = addr.kernel_to_physical().unwrap();
			e.store(to_entry(addr, flags), Relaxed);
		});
		// Set new entry
		let addr = VirtAddr::from(new_table).kernel_to_physical().unwrap();
		self[index].store(to_entry(addr, flags), Relaxed);
	}

	/// Tells whether the table at index `index` in the page directory is empty.
	pub fn is_empty(&self) -> bool {
		// TODO Use a counter instead. Increment it when mapping a page in the table and
		// decrement it when unmapping. Then return `true` if the counter has the value
		// `0`
		self.iter().all(|e| e.load(Relaxed) & FLAG_PRESENT == 0)
	}
}

/// Kernel space paging tables common to every context.
static KERNEL_TABLES: [Table; KERNELSPACE_TABLES] = unsafe { mem::zeroed() };

/// Allocates a table and returns its virtual address.
///
/// If the allocation fails, the function returns an error.
fn alloc_table() -> NonNull<Table> {
	// The allocation cannot fail thanks to `BUDDY_RETRY`
	let mut table = buddy::alloc_kernel(0, BUDDY_RETRY).unwrap().cast::<Table>();
	unsafe {
		table.as_mut().fill_with(AtomicUsize::default);
	}
	table
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
fn to_entry(addr: PhysAddr, flags: usize) -> usize {
	// Sanitize flags
	let flags = flags & FLAGS_MASK | FLAG_PRESENT;
	// Address alignment guarantees the address does not overlap flags
	addr.0 | flags
}

/// Turns an entry back into an object/flags pair.
///
/// # Safety
///
/// If the object's address in the entry is invalid, the behaviour is undefined.
#[inline]
unsafe fn unwrap_entry(entry: usize) -> (NonNull<Table>, usize) {
	let table = PhysAddr(entry & ADDR_MASK)
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
pub fn alloc() -> NonNull<Table> {
	let mut ctx = alloc_table();
	// Init kernel entries
	let ctx_ref = unsafe { ctx.as_mut() };
	KERNEL_TABLES
		.iter()
		.zip(ctx_ref[USERSPACE_TABLES..].iter_mut())
		.for_each(|(src, dst)| {
			let addr = src as *const Table;
			let addr = VirtAddr::from(addr).kernel_to_physical().unwrap();
			*dst.get_mut() = to_entry(addr, KERNEL_FLAGS);
		});
	ctx
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

fn translate_impl(mut table: &Table, addr: VirtAddr) -> Option<usize> {
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(addr, level);
		let entry = table[index].load(Relaxed);
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
		let phys_addr = PhysAddr(entry & ADDR_MASK);
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
	let physaddr = (entry & ADDR_MASK) | (addr.0 & remain_mask);
	Some(PhysAddr(physaddr))
}

/// Tells whether a table may be freed if empty.
fn can_remove_table(level: usize, index: usize) -> bool {
	(1..(DEPTH - 1)).contains(&level) || (level == DEPTH - 1 && index < USERSPACE_TABLES)
}

/// Inner implementation of [`crate::memory::vmem::VMem::map`] for x86.
///
/// # Safety
///
/// In case the mapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub unsafe fn map(mut table: &mut Table, physaddr: PhysAddr, virtaddr: VirtAddr, flags: usize) {
	// Sanitize
	let physaddr = PhysAddr(physaddr.0 & !(PAGE_SIZE - 1));
	let virtaddr = VirtAddr(virtaddr.0 & !(PAGE_SIZE - 1));
	// TODO support FLAG_PAGE_SIZE (requires a way to specify a which level it must be enabled)
	let flags = (flags & FLAGS_MASK & !FLAG_PAGE_SIZE) | FLAG_PRESENT;
	// Set entries
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(virtaddr, level);
		let previous = table[index].load(Relaxed);
		if level == 0 {
			table[index].store(to_entry(physaddr, flags), Relaxed);
			break;
		}
		#[cfg(target_arch = "x86_64")]
		let flags = flags & !FLAG_XD;
		// Allocate a table if necessary
		if previous & FLAG_PRESENT == 0 {
			// No table is present, allocate one
			let new_table = alloc_table();
			let addr = VirtAddr::from(new_table).kernel_to_physical().unwrap();
			table[index].store(to_entry(addr, flags), Relaxed);
		} else if previous & FLAG_PAGE_SIZE != 0 {
			// A PSE entry is present, need to expand it for the mapping
			table.expand(index);
		}
		table[index].fetch_or(flags, Relaxed);
		// Jump to next table
		let entry = table[index].load(Relaxed);
		table = unsafe { unwrap_entry(entry).0.as_mut() };
	}
}

/// Inner implementation of [`crate::memory::vmem::VMem::unmap`] for x86.
///
/// # Safety
///
/// In case the unmapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub unsafe fn unmap(mut table: &mut Table, virtaddr: VirtAddr) {
	// Sanitize
	let virtaddr = VirtAddr(virtaddr.0 & !(PAGE_SIZE - 1));
	// Read entries
	let mut tables: [Option<(NonNull<Table>, usize)>; DEPTH] = [None; DEPTH];
	for level in (0..DEPTH).rev() {
		let index = get_addr_element_index(virtaddr, level);
		let entry = table[index].load(Relaxed);
		tables[level] = Some((NonNull::from(table), index));
		// If the entry does not exist or is PSE, stop here
		if entry & FLAG_PRESENT == 0 || entry & FLAG_PAGE_SIZE != 0 {
			break;
		}
		// Jump to next table
		table = unsafe { unwrap_entry(entry).0.as_mut() };
	}
	// Remove entry and go up to remove tables that are now empty
	for t in tables {
		let Some((mut table, index)) = t else {
			continue;
		};
		let table = unsafe { table.as_mut() };
		table[index].store(0, Relaxed);
		if !table.is_empty() {
			break;
		}
	}
}

/// Inner implementation of [`crate::memory::vmem::VMem::poll_dirty`] for x86.
pub fn poll_dirty(table: &Table, virtaddr: VirtAddr) -> Option<(PhysAddr, bool)> {
	let entry = translate_impl(table, virtaddr)?;
	let physaddr = PhysAddr(entry & ADDR_MASK);
	Some((physaddr, entry & FLAG_DIRTY != 0))
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

unsafe fn free_impl(mut page_dir: NonNull<Table>, depth: usize) {
	if depth < DEPTH - 1 {
		let pd = unsafe { page_dir.as_mut() };
		let max = if depth == 0 {
			USERSPACE_TABLES
		} else {
			ENTRIES_PER_TABLE
		};
		for entry in &pd[..max] {
			let entry = entry.load(Relaxed);
			let (table, flags) = unwrap_entry(entry);
			if flags & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT {
				free_impl(table, depth + 1);
			}
		}
	}
	free_table(page_dir);
}

/// Destroys the given page directory, including its children elements.
///
/// # Safety
///
/// It is assumed the context is not being used.
///
/// Subsequent uses of `page_dir` are undefined.
pub unsafe fn free(page_dir: NonNull<Table>) {
	free_impl(page_dir, 0);
}

/// Prepares for virtual memory management on the current CPU.
pub(crate) fn prepare() {
	// Set cr4 flags
	// Enable GLOBAL flag
	let mut cr4 = register_get!("cr4") | (1 << 7);
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
