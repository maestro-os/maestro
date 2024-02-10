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
	errno::AllocResult,
	memory,
	memory::{buddy, vmem::VMem},
	register_get, register_set,
	util::{down_align, lock::Mutex, TryClone},
};
use core::{arch::asm, ffi::c_void, ptr::null_mut};

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
const ENTRIES_PER_TABLE: usize = 1024;
/// TODO doc
type Table = [u32; ENTRIES_PER_TABLE];

/// Enables paging with the given page directory.
///
/// # Safety
///
/// The caller must ensure the given page directory is correct.
/// Meaning it must be mapping the kernel's code and data sections, and any regions of memory that
/// might be accessed in the future.
pub(super) unsafe fn enable_paging(directory: *const u32) {
	asm!(
		"mov cr3, {dir}",
		"mov {tmp}, cr0",
		"or {tmp}, 0x80010000",
		"mov cr0, {tmp}",
		dir = in(reg) directory,
		tmp = out(reg) _,
	)
}

/// Kernel space paging tables common to every contexts.
static KERNEL_TABLES: Mutex<[*mut Table; 256]> = Mutex::new([null_mut(); 256]);

/// Allocates a table and returns its virtual address.
///
/// If the allocation fails, the function returns an error.
fn alloc_table() -> AllocResult<&'static mut Table> {
	let table = unsafe { buddy::alloc_kernel(0)?.cast::<Table>().as_mut() };
	table.fill(0);
	Ok(table)
}

/// Frees a table.
///
/// # Safety
///
/// Further accesses to the table after this function are undefined.
unsafe fn free_table(table: &Table) {
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
unsafe fn unwrap_entry(entry: u32) -> (&'static mut Table, u32) {
	let table_addr = (entry & ADDR_MASK) as *mut Table;
	let table = &mut *(memory::kern_to_virt(table_addr) as *mut _);
	let flags = entry & FLAGS_MASK;
	(table, flags)
}

/// A virtual memory context handler for the x86 architecture.
#[derive(Debug)]
pub struct X86VMem {
	/// The virtual address to the page directory.
	page_dir: &'static mut Table,
}

/// Page tables manipulation.
mod table {
	use super::*;

	/// Creates an empty page table at index `index` of the given table `parent`.
	pub fn create(parent: &mut Table, index: usize, flags: u32) -> AllocResult<()> {
		// Sanitize flags
		let flags = (flags & !FLAG_PAGE_SIZE) | FLAG_PRESENT;
		// Allocate table
		let table = {
			if index < 768 {
				alloc_table()?
			} else {
				KERNEL_TABLES.lock()[index - 768]
			}
		};
		parent[index] = to_entry(table, flags);
		Ok(())
	}

	/// Expands a large block into a page table.
	///
	/// This function allocates a new page table and fills it so that the memory mapping keeps the
	/// same behavior.
	pub fn expand(parent: &mut Table, index: usize) -> AllocResult<()> {
		let entry = parent[index];
		if entry & FLAG_PRESENT == 0 || entry & FLAG_PAGE_SIZE == 0 {
			return Ok(());
		}
		// New flags
		let flags = (entry & FLAGS_MASK) & !FLAG_PAGE_SIZE;
		// Create table
		create(parent, index, flags)?;
		let entry = parent[index];
		let base_addr = (entry & ADDR_MASK) as usize;
		let (table, _) = unsafe { unwrap_entry(entry) };
		for (i, e) in table[0..ENTRIES_PER_TABLE].iter_mut().enumerate() {
			let addr = (base_addr + (i * memory::PAGE_SIZE)) as *const c_void;
			*e = to_entry(addr, flags);
		}
		Ok(())
	}

	/// Tells whether the table at index `index` in the page directory is empty.
	pub fn is_empty(table: &Table) -> bool {
		// TODO Use a counter instead. Increment it when mapping a page in the table and
		// decrement it when unmapping. Then return `true` if the counter has the value
		// `0`
		table.iter().all(|e| e & FLAG_PRESENT == 0)
	}

	/// Deletes the entry (and the underlying table if present) at index `index` in the given
	/// `table`.
	pub fn delete(table: &mut Table, index: usize) {
		let entry_val = table[index];
		unsafe {
			let (entry, _) = unwrap_entry(entry_val);
			free_table(entry);
		}
		table[index] = 0;
	}
}

impl X86VMem {
	/// Asserts that the map operation shall not result in a crash.
	#[cfg(config_debug_debug)]
	fn check_map(&self, virt_ptr: *const c_void, phys_ptr: *const c_void, pse: bool) {
		if !self.is_bound() {
			return;
		}
		let esp = unsafe { register_get!("esp") as *const c_void };
		let aligned_stack_page = crate::util::down_align(esp, memory::PAGE_SIZE);
		let size = if pse { ENTRIES_PER_TABLE } else { 1 };
		for i in 0..size {
			let virt_ptr = unsafe { virt_ptr.add(i * memory::PAGE_SIZE) };
			if virt_ptr != aligned_stack_page {
				return;
			}
			assert_eq!(self.translate(aligned_stack_page), Some(phys_ptr));
		}
	}

	/// Asserts that the unmap operation shall not result in a crash.
	#[cfg(config_debug_debug)]
	fn check_unmap(&self, virt_ptr: *const c_void, pse: bool) {
		if !self.is_bound() {
			return;
		}
		let esp = unsafe { register_get!("esp") as *const c_void };
		let aligned_stack_page = crate::util::down_align(esp, memory::PAGE_SIZE);
		let size = if pse { ENTRIES_PER_TABLE } else { 1 };
		for i in 0..size {
			let virt_ptr = unsafe { virt_ptr.add(i * memory::PAGE_SIZE) };
			if virt_ptr != aligned_stack_page {
				return;
			}
			assert_ne!(virt_ptr, aligned_stack_page);
		}
	}

	/// Initializes a new page directory.
	///
	/// The kernel memory is mapped into the context by default.
	pub fn new() -> AllocResult<Self> {
		let vmem = Self {
			page_dir: alloc_table()?,
		};
		// Init kernel entries
		let kernel_tables = KERNEL_TABLES.lock();
		let flags = FLAG_PRESENT | FLAG_WRITE | FLAG_USER | FLAG_GLOBAL;
		vmem.page_dir[768..]
			.iter_mut()
			.zip(kernel_tables.iter())
			.for_each(|(dst, src)| {
				*dst = to_entry(*src, flags);
			});
		Ok(vmem)
	}

	/// Returns the index of the element corresponding to the given virtual
	/// address `ptr` for element at level `level` in the tree.
	///
	/// The level represents the depth in the tree. `0` is the deepest.
	#[inline]
	fn get_addr_element_index(ptr: *const c_void, level: usize) -> usize {
		((ptr as usize) >> (12 + level * 10)) & 0x3ff
	}

	/// Returns the paging entry for the given pointer.
	///
	/// If no entry is found (the present flag is not set), `None` is returned.
	///
	/// The function searches in tables recursively and returns the deepest entry.
	fn resolve(&self, ptr: *const c_void) -> Option<&u32> {
		// First level
		let index = Self::get_addr_element_index(ptr, 1);
		let entry = &self.page_dir[index];
		if entry & FLAG_PRESENT == 0 {
			return None;
		}
		if entry & FLAG_PAGE_SIZE != 0 {
			return Some(entry);
		}
		// Second level
		let index = Self::get_addr_element_index(ptr, 0);
		let (table, _) = unsafe { unwrap_entry(*entry) };
		let entry = &table[index];
		if entry & FLAG_PRESENT == 0 {
			return None;
		}
		Some(entry)
	}

	/// Tells whether to use PSE mapping for the given virtual address `addr`
	/// and remaining pages `pages`.
	fn use_pse(addr: *const c_void, pages: usize) -> bool {
		// The end address of the hypothetical PSE block
		let Some(pse_end) = (addr as usize).checked_add(ENTRIES_PER_TABLE * memory::PAGE_SIZE)
		else {
			// Overflow
			return false;
		};
		// Ensure no PSE block is created in kernel space
		pse_end < (memory::PROCESS_END as usize)
		// Check the address is aligned on the PSE boundary
			&& addr.is_aligned_to(ENTRIES_PER_TABLE * memory::PAGE_SIZE)
		// Check that there remain enough pages to make a PSE block
			&& pages >= ENTRIES_PER_TABLE
	}

	/// Maps the given physical address `physaddr` to the given virtual address
	/// `virtaddr` with the given flags using blocks of 1024 pages (PSE).
	fn map_pse(&mut self, physaddr: *const c_void, virtaddr: *const c_void, flags: u32) {
		// Sanitize flags
		let flags = (flags & FLAGS_MASK) | FLAG_PRESENT | FLAG_PAGE_SIZE;
		let index = Self::get_addr_element_index(virtaddr, 1);
		let entry_val = self.page_dir[index];
		if index < 768 && entry_val & FLAG_PRESENT != 0 && entry_val & FLAG_PAGE_SIZE == 0 {
			table::delete(self.page_dir, index);
		}
		self.page_dir[index] = to_entry(physaddr, flags);
		self.invalidate_page(virtaddr);
	}

	/// Unmaps the large block (PSE) at the given virtual address `virtaddr`.
	fn unmap_pse(&mut self, virtaddr: *const c_void) {
		let index = Self::get_addr_element_index(virtaddr, 1);
		let entry_val = self.page_dir[index];
		if entry_val & FLAG_PRESENT == 0 || entry_val & FLAG_PAGE_SIZE == 0 {
			return;
		}
		self.page_dir[index] = 0;
	}
}

impl VMem for X86VMem {
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void> {
		let entry = self.resolve(ptr)?;
		let remain_mask = if entry & FLAG_PAGE_SIZE == 0 {
			memory::PAGE_SIZE - 1
		} else {
			ENTRIES_PER_TABLE * memory::PAGE_SIZE - 1
		};
		let mut virtptr = (entry & ADDR_MASK) as usize;
		virtptr |= ptr as usize & remain_mask;
		Some(virtptr as _)
	}

	unsafe fn map(
		&mut self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		flags: u32,
	) -> AllocResult<()> {
		#[cfg(config_debug_debug)]
		self.check_map(virtaddr, physaddr, false);
		// Sanitize
		let physaddr = down_align(physaddr, memory::PAGE_SIZE);
		let virtaddr = down_align(physaddr, memory::PAGE_SIZE);
		let flags = (flags & FLAGS_MASK) | FLAG_PRESENT;
		// Handle first level
		let index = Self::get_addr_element_index(virtaddr, 1);
		let entry_val = self.page_dir[index];
		if entry_val & FLAG_PRESENT == 0 {
			table::create(self.page_dir, index, flags)?;
		} else if entry_val & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, index)?;
		}
		if index < 768 {
			// Set the table's flags
			self.page_dir[index] |= flags;
		}
		// Set new entry
		let entry_val = self.page_dir[index];
		debug_assert!(entry_val & FLAG_PAGE_SIZE == 0);
		let (table, _) = unsafe { unwrap_entry(entry_val) };
		let index = Self::get_addr_element_index(virtaddr, 0);
		table[index] = to_entry(physaddr, flags);
		self.invalidate_page(virtaddr);
		Ok(())
	}

	unsafe fn map_range(
		&mut self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		pages: usize,
		flags: u32,
	) -> AllocResult<()> {
		debug_assert!(
			(virtaddr as usize / memory::PAGE_SIZE) + pages
				<= (usize::MAX / memory::PAGE_SIZE) + 1
		);
		let flags = flags & FLAGS_MASK;
		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let next_physaddr = ((physaddr as usize) + off) as *const c_void;
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;
			if Self::use_pse(next_virtaddr, pages - i) {
				#[cfg(config_debug_debug)]
				self.check_map(next_virtaddr, next_physaddr, true);
				self.map_pse(next_physaddr, next_virtaddr, flags);
				i += ENTRIES_PER_TABLE;
			} else {
				self.map(next_physaddr, next_virtaddr, flags)?;
				i += 1;
			}
		}
		Ok(())
	}

	unsafe fn unmap(&mut self, virtaddr: *const c_void) -> AllocResult<()> {
		#[cfg(config_debug_debug)]
		self.check_unmap(virtaddr, false);
		// Sanitize
		let virtaddr = down_align(virtaddr, memory::PAGE_SIZE);
		// Handle first level
		let index = Self::get_addr_element_index(virtaddr, 1);
		let entry_val = self.page_dir[index];
		if entry_val & FLAG_PRESENT == 0 {
			return Ok(());
		}
		if entry_val & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, index)?;
		}
		// Remove the table if it is empty and if not a kernel space table
		let (table, _) = unwrap_entry(entry_val);
		if index < 768 && table::is_empty(table) {
			table::delete(self.page_dir, index);
		}
		// Remove entry
		let index = Self::get_addr_element_index(virtaddr, 0);
		table[index] = 0;
		self.invalidate_page(virtaddr);
		Ok(())
	}

	unsafe fn unmap_range(&mut self, virtaddr: *const c_void, pages: usize) -> AllocResult<()> {
		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!((virtaddr as usize) + (pages * memory::PAGE_SIZE) >= (virtaddr as usize));
		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;
			// Check whether the page is mapped in PSE
			let index = Self::get_addr_element_index(virtaddr, 1);
			let entry_val = self.page_dir[index];
			let is_pse =
				(entry_val & FLAG_PAGE_SIZE != 0) && Self::use_pse(next_virtaddr, pages - i);
			if is_pse {
				#[cfg(config_debug_debug)]
				self.check_unmap(virtaddr, true);
				self.unmap_pse(next_virtaddr);
				i += ENTRIES_PER_TABLE;
			} else {
				self.unmap(next_virtaddr)?;
				i += 1;
			}
		}
		Ok(())
	}

	unsafe fn bind(&self) {
		if !self.is_bound() {
			unsafe {
				enable_paging(memory::kern_to_phys(self.page_dir.as_ptr() as _) as _);
			}
		}
	}

	#[inline]
	fn is_bound(&self) -> bool {
		unsafe { register_get!("cr3") == memory::kern_to_phys(self.page_dir.as_ptr() as _) as _ }
	}

	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	fn invalidate_page(&self, addr: *const c_void) {
		if !self.is_bound() {
			return;
		}
		// TODO Also invalidate on other CPU core (TLB shootdown)
		unsafe {
			asm!("invlpg [{addr}]", addr = in(reg) addr);
		}
	}

	fn flush(&self) {
		if !self.is_bound() {
			return;
		}
		// TODO Also invalidate on other CPU core (TLB shootdown)
		// Flush TLB
		unsafe {
			asm!(
				"mov {tmp}, cr3",
				"mov cr3, {tmp}",
				tmp = out(reg) _
			);
		}
	}
}

impl TryClone for X86VMem {
	fn try_clone(&self) -> AllocResult<Self> {
		let s = Self {
			page_dir: alloc_table()?,
		};
		for i in 0..ENTRIES_PER_TABLE {
			let entry_val = self.page_dir[i];
			let (src_table, src_flags) = unsafe { unwrap_entry(entry_val) };
			if src_flags & FLAG_PRESENT == 0 {
				continue;
			}
			if src_flags & FLAG_PAGE_SIZE == 0 {
				let dest_table = {
					if i < 768 {
						let dest_table = alloc_table()?;
						dest_table.copy_from_slice(src_table);
						dest_table
					} else {
						KERNEL_TABLES.lock()[i - 768]
					}
				};
				s.page_dir[i] = to_entry(dest_table, src_flags);
			} else {
				s.page_dir[i] = entry_val;
			}
		}
		Ok(s)
	}
}

impl Drop for X86VMem {
	/// Destroys the given page directory, including its children elements.
	///
	/// If the page directory is being used, the kernel shall panic.
	fn drop(&mut self) {
		if self.is_bound() {
			panic!("Dropping virtual memory context handler while in use!");
		}
		for i in 0..768 {
			let (table, flags) = unsafe { unwrap_entry(self.page_dir[i]) };
			if flags & FLAG_PRESENT != 0 && flags & FLAG_PAGE_SIZE == 0 {
				unsafe {
					free_table(table);
				}
			}
		}
		unsafe {
			free_table(self.page_dir);
		}
	}
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
		*table = alloc_table()?;
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::tty::vga;

	#[test_case]
	fn vmem_x86_vga_text_access() {
		let vmem = X86VMem::new().unwrap();
		let len = vga::WIDTH as usize * vga::HEIGHT as usize;
		for i in 0..len {
			let ptr = unsafe { vga::get_buffer_virt().add(i) };
			vmem.translate(ptr as _).unwrap();
		}
	}
}
