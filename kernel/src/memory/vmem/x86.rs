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

use crate::{memory, memory::buddy, register_get, register_set};
use core::{
	arch::asm,
	ffi::c_void,
	ptr::{null_mut, NonNull},
};
use utils::{down_align, errno::AllocResult, lock::Mutex};

/// x86 paging flag. If set, prevents the CPU from updating the associated
/// addresses when the TLB is flushed.
pub const FLAG_GLOBAL: u32 = 0b100000000;
/// x86 paging flag. If set, pages are 4 MB long.
pub const FLAG_PAGE_SIZE: u32 = 0b010000000;
/// x86 paging flag. Indicates that the page has been written.
pub const FLAG_DIRTY: u32 = 0b001000000;
/// x86 paging flag. Set if the page has been read or written.
pub const FLAG_ACCESSED: u32 = 0b000100000;
/// x86 paging flag. If set, page will not be cached.
pub const FLAG_CACHE_DISABLE: u32 = 0b000010000;
/// x86 paging flag. If set, write-through caching is enabled.
/// If not, then write-back is enabled instead.
pub const FLAG_WRITE_THROUGH: u32 = 0b000001000;
/// x86 paging flag. If set, the page can be accessed by userspace operations.
pub const FLAG_USER: u32 = 0b000000100;
/// x86 paging flag. If set, the page can be written.
pub const FLAG_WRITE: u32 = 0b000000010;
/// x86 paging flag. If set, the page is present.
pub const FLAG_PRESENT: u32 = 0b000000001;

/// Flags mask in a page directory entry.
pub const FLAGS_MASK: u32 = 0xfff;
/// Address mask in a page directory entry. The address doesn't need every byte
/// since it must be page-aligned.
pub const ADDR_MASK: u32 = !FLAGS_MASK;

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
pub(super) const ENTRIES_PER_TABLE: usize = 1024;
/// The number of tables reserved for the userspace.
///
/// Those tables start at the beginning of the page directory. Remaining tables are reserved for
/// the kernel.
pub(super) const USERSPACE_TABLES: usize = 768;
/// Paging table.
pub(super) type Table = [u32; ENTRIES_PER_TABLE];
/// Kernel space entries flags.
const KERNEL_FLAGS: u32 = FLAG_PRESENT | FLAG_WRITE | FLAG_USER | FLAG_GLOBAL;

/// Kernel space paging tables common to every context.
static KERNEL_TABLES: Mutex<[*mut Table; 256]> =
	Mutex::new([null_mut(); ENTRIES_PER_TABLE - USERSPACE_TABLES]);

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
fn to_entry<T>(table: *const T, flags: u32) -> u32 {
	// Pointer alignment guarantees the address does not overlap flags
	let physaddr = memory::kern_to_phys(table) as u32;
	// Sanitize flags
	let flags = flags & FLAGS_MASK | FLAG_PRESENT;
	physaddr | flags
}

/// Turns an entry back into a object/flags pair.
///
/// # Safety
///
/// If the object's address in the entry is invalid, the behaviour is undefined.
#[inline]
unsafe fn unwrap_entry(entry: u32) -> (NonNull<Table>, u32) {
	let table_addr = (entry & ADDR_MASK) as *mut Table;
	let table = NonNull::from(&mut *(memory::kern_to_virt(table_addr) as *mut _));
	let flags = entry & FLAGS_MASK;
	(table, flags)
}

/// Page tables manipulation.
mod table {
	use super::*;

	/// Creates an expanded table meant to replace a PSE entry.
	///
	/// This function allocates a new page table and fills it so that the memory mapping keeps the
	/// same behavior.
	pub fn expand(parent: &mut Table, index: usize) -> AllocResult<()> {
		let entry = parent[index];
		if entry & FLAG_PRESENT == 0 || entry & FLAG_PAGE_SIZE == 0 {
			return Ok(());
		}
		// Sanitize
		let flags = (entry & FLAGS_MASK) & !FLAG_PAGE_SIZE;
		// Create table
		let new_table = alloc_table()?;
		let base_addr = new_table.as_ptr() as usize;
		let table = unsafe { unwrap_entry(entry).0.as_mut() };
		table.iter_mut().enumerate().for_each(|(i, e)| {
			let addr = (base_addr + i * memory::PAGE_SIZE) as *const c_void;
			*e = to_entry(addr, flags);
		});
		Ok(())
	}

	/// Tells whether the table at index `index` in the page directory is empty.
	pub fn is_empty(table: &Table) -> bool {
		// TODO Use a counter instead. Increment it when mapping a page in the table and
		// decrement it when unmapping. Then return `true` if the counter has the value
		// `0`
		table.iter().all(|e| e & FLAG_PRESENT == 0)
	}
}

/// Allocates and initializes a new page directory.
///
/// The kernel memory is mapped into the context by default.
pub(super) fn alloc() -> AllocResult<NonNull<Table>> {
	let mut page_dir = alloc_table()?;
	// Init kernel entries
	let kernel_tables = KERNEL_TABLES.lock();
	let pd = unsafe { page_dir.as_mut() };
	pd[USERSPACE_TABLES..]
		.iter_mut()
		.zip(kernel_tables.iter())
		.for_each(|(dst, src)| {
			*dst = to_entry(*src, KERNEL_FLAGS);
		});
	Ok(page_dir)
}

/// Returns the index of the element corresponding to the given virtual
/// address `ptr` for element at level `level` in the tree.
///
/// The level represents the depth in the tree. `0` is the deepest.
#[inline]
fn get_addr_element_index(ptr: *const c_void, level: usize) -> usize {
	((ptr as usize) >> (12 + level * 10)) & 0x3ff
}

/// Returns the corresponding entry for [`translate`].
fn translate_impl(page_dir: &Table, ptr: *const c_void) -> Option<u32> {
	// First level
	let index = get_addr_element_index(ptr, 1);
	let entry_val = page_dir[index];
	if entry_val & FLAG_PRESENT == 0 {
		return None;
	}
	if entry_val & FLAG_PAGE_SIZE != 0 {
		return Some(entry_val);
	}
	// Second level
	let index = get_addr_element_index(ptr, 0);
	let table = unsafe { unwrap_entry(entry_val).0.as_mut() };
	let entry_val = table[index];
	if entry_val & FLAG_PRESENT == 0 {
		return None;
	}
	Some(entry_val)
}

/// Translates the given virtual address to the corresponding physical address using `page_dir`.
pub(super) fn translate(page_dir: &Table, ptr: *const c_void) -> Option<*const c_void> {
	let entry = translate_impl(page_dir, ptr)?;
	let remain_mask = if entry & FLAG_PAGE_SIZE == 0 {
		memory::PAGE_SIZE - 1
	} else {
		ENTRIES_PER_TABLE * memory::PAGE_SIZE - 1
	};
	let mut virtptr = (entry & ADDR_MASK) as usize;
	virtptr |= ptr as usize & remain_mask;
	Some(virtptr as _)
}

/// Inner version of [`super::Rollback`] for x86.
pub(super) struct Rollback {
	/// The virtual address of the affected page.
	virtaddr: *const c_void,
	/// Previous entry in the page table, unless the [`FLAG_PAGE_SIZE`] flag is set, in which case
	/// the entry is the page directory.
	previous_entry: u32,
	/// The table that was deleted, if any.
	table: Option<NonNull<Table>>,
}

impl Rollback {
	/// Rollbacks the operation on `page_dir`.
	#[cold]
	pub(super) fn rollback(mut self, page_dir: &mut Table) {
		let index = get_addr_element_index(self.virtaddr, 1);
		// Replace the table for the previous one
		if let Some(table) = self.table.take() {
			let flags = self.previous_entry & FLAGS_MASK & !FLAG_PAGE_SIZE;
			page_dir[index] = to_entry(table.as_ptr(), flags);
			// No need to care about the previous table as the algorithms will never replace an
			// already present table
		}
		// If the table is PSE, simply replace the entry and stop here
		if self.previous_entry & FLAG_PAGE_SIZE != 0 {
			page_dir[index] = self.previous_entry;
			return;
		}
		// If no table is present, stop here
		if page_dir[index] & FLAG_PRESENT == 0 {
			return;
		}
		// A table is present, set entry with previous value
		let mut table_ptr = unsafe { unwrap_entry(page_dir[index]).0 };
		let table = unsafe { table_ptr.as_mut() };
		let index = get_addr_element_index(self.virtaddr, 0);
		table[index] = self.previous_entry;
		// If the table is now empty, delete it
		// `is_empty` is expensive. Call it only if the entry has been set to "not present"
		if table[index] & FLAG_PRESENT == 0 && table::is_empty(table) {
			// The table will be freed when dropping `self`
			self.table = Some(table_ptr);
			page_dir[index] = 0;
		}
	}
}

impl Drop for Rollback {
	fn drop(&mut self) {
		if let Some(table) = self.table {
			unsafe {
				free_table(table);
			}
		}
	}
}

/// Inner implementation of [`super::VMem::map`] for x86.
///
/// # Safety
///
/// In case the mapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub(super) unsafe fn map(
	page_dir: &mut Table,
	physaddr: *const c_void,
	virtaddr: *const c_void,
	flags: u32,
) -> AllocResult<Rollback> {
	// Sanitize
	let physaddr = down_align(physaddr, memory::PAGE_SIZE);
	let virtaddr = down_align(virtaddr, memory::PAGE_SIZE);
	let flags = (flags & FLAGS_MASK) | FLAG_PRESENT;
	// First level
	let pd_index = get_addr_element_index(virtaddr, 1);
	let mut previous_entry = page_dir[pd_index];
	// If using PSE, set entry and stop
	if flags & FLAG_PAGE_SIZE != 0 {
		page_dir[pd_index] = to_entry(physaddr, flags);
		let table = (previous_entry & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT)
			.then(|| unsafe { unwrap_entry(previous_entry).0 });
		return Ok(Rollback {
			virtaddr,
			previous_entry,
			table,
		});
	}
	let mut expanded = false;
	if previous_entry & FLAG_PRESENT == 0 {
		// No table is present, allocate one
		let table = alloc_table()?;
		page_dir[pd_index] = to_entry(table.as_ptr(), flags);
	} else if previous_entry & FLAG_PAGE_SIZE != 0 {
		// A PSE entry is present, need to expand it for the mapping
		table::expand(page_dir, pd_index)?;
		expanded = true;
	}
	// Set the table's flags
	page_dir[pd_index] |= flags;
	// Second level
	let table = unsafe { unwrap_entry(page_dir[pd_index]).0.as_mut() };
	let table_index = get_addr_element_index(virtaddr, 0);
	if !expanded {
		previous_entry = table[table_index];
	}
	table[table_index] = to_entry(physaddr, flags);
	Ok(Rollback {
		virtaddr,
		previous_entry,
		table: None,
	})
}

/// Inner implementation of [`super::VMem::unmap`] for x86.
///
/// # Safety
///
/// In case the unmapped memory is in kernelspace, the caller must ensure the code and stack of the
/// kernel remain accessible and valid.
pub(super) unsafe fn unmap(
	page_dir: &mut Table,
	virtaddr: *const c_void,
) -> AllocResult<Rollback> {
	// Sanitize
	let virtaddr = down_align(virtaddr, memory::PAGE_SIZE);
	// First level
	let pd_index = get_addr_element_index(virtaddr, 1);
	let mut previous_entry = page_dir[pd_index];
	if previous_entry & FLAG_PRESENT == 0 {
		// The entry does not exist, do nothing
		return Ok(Rollback {
			virtaddr,
			previous_entry,
			table: None,
		});
	}
	if previous_entry & FLAG_PAGE_SIZE != 0 {
		// The entry is PSE, remove it and stop here
		page_dir[pd_index] = 0;
		return Ok(Rollback {
			virtaddr,
			previous_entry,
			table: None,
		});
	}
	let mut table_ptr = unsafe { unwrap_entry(previous_entry).0 };
	let table = unsafe { table_ptr.as_mut() };
	// Second level
	let table_index = get_addr_element_index(virtaddr, 0);
	previous_entry = table[table_index];
	table[table_index] = 0;
	// Remove the table if it is empty and if not a kernel space table
	let table = if table_index < USERSPACE_TABLES
		&& previous_entry & FLAG_PRESENT != 0
		&& table::is_empty(table)
	{
		page_dir[pd_index] = 0;
		Some(table_ptr)
	} else {
		None
	};
	Ok(Rollback {
		virtaddr,
		previous_entry,
		table,
	})
}

/// Binds the given page directory to the current CPU.
///
/// If paging is not enabled, the function enables it.
///
/// # Safety
///
/// The caller must ensure the given page directory is correct.
/// Meaning it must be mapping the kernel's code and data sections, and any regions of memory that
/// might be accessed in the future.
pub(super) unsafe fn bind(page_dir: *const c_void) {
	asm!(
		"mov cr3, {dir}",
		"mov {tmp}, cr0",
		"or {tmp}, 0x80010000",
		"mov cr0, {tmp}",
		dir = in(reg) page_dir,
		tmp = out(reg) _,
	)
}

/// Tells whether the given page directory is bound on the current CPU.
#[inline]
pub(super) fn is_bound(page_dir: NonNull<Table>) -> bool {
	let physaddr = memory::kern_to_phys(page_dir.as_ptr() as _) as _;
	unsafe { register_get!("cr3") == physaddr }
}

/// Invalidate the page at the given address on the current CPU.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub(super) fn invalidate_page_current(addr: *const c_void) {
	unsafe {
		asm!("invlpg [{addr}]", addr = in(reg) addr);
	}
}

/// Flush the Translation Lookaside Buffer (TLB) on the current CPU.
pub(super) fn flush_current() {
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
pub(super) unsafe fn free(mut page_dir: NonNull<Table>) {
	let pd = unsafe { page_dir.as_mut() };
	for entry in &pd[..USERSPACE_TABLES] {
		let (table, flags) = unwrap_entry(*entry);
		if flags & (FLAG_PRESENT | FLAG_PAGE_SIZE) == FLAG_PRESENT {
			free_table(table);
		}
	}
	free_table(page_dir);
}

/// Initializes virtual memory management.
pub(super) fn init() -> AllocResult<()> {
	// Enable GLOBAL flag
	unsafe {
		let cr4 = register_get!("cr4") | 0b10000000;
		register_set!("cr4", cr4);
	}
	// Allocate kernel tables
	let mut tables = KERNEL_TABLES.lock();
	for table in &mut *tables {
		*table = alloc_table()?.as_ptr();
	}
	Ok(())
}
