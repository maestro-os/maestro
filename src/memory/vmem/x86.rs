//! x86 virtual memory works with a tree structure. Each element is an array of
//! subelements. The position of the elements in the arrays allows to tell the
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
//! Each entries of elements contains the physical address to the element/page and some flags.
//! The flags can be stored with the address in only 4 bytes large entries because addresses have
//! to be page-aligned, freeing 12 bits in the entry for the flags.
//!
//! For each entries of each elements, the kernel must keep track of how many
//! elements are being used. This can be done with a simple counter: when an
//! entry is allocated, the counter is incremented and when an entry is freed,
//! the counter is decremented. When the counter reaches 0, the element can be
//! freed.
//!
//! The Page Size Extension (PSE) allows to map 4MB large blocks without using a
//! page table.

use crate::cpu;
use crate::errno::Errno;
use crate::memory;
use crate::memory::buddy;
use crate::memory::vmem::VMem;
use crate::util::lock::Mutex;
use crate::util::TryClone;
use core::ffi::c_void;
use core::ptr;
use core::slice;

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
/// x86 paging flag. If set, the page can be wrote.
pub const FLAG_WRITE: u32 = 0b000000010;
/// x86 paging flag. If set, the page is present.
pub const FLAG_PRESENT: u32 = 0b000000001;

/// Flags mask in a page directory entry.
pub const FLAGS_MASK: u32 = 0xfff;
/// Address mask in a page directory entry. The address doesn't need every bytes
/// since it must be page-aligned.
pub const ADDR_MASK: u32 = !FLAGS_MASK;

/// x86 page fault flag. If set, the page was present.
pub const PAGE_FAULT_PRESENT: u32 = 0b00001;
/// x86 page fault flag. If set, the error was caused by a write operation, else
/// the error was caused by a read operation.
pub const PAGE_FAULT_WRITE: u32 = 0b00010;
/// x86 page fault flag. If set, the page fault was caused by a userspace
/// operation.
pub const PAGE_FAULT_USER: u32 = 0b00100;
/// x86 page fault flag. If set, one or more page directory entries contain
/// reserved bits which are set.
pub const PAGE_FAULT_RESERVED: u32 = 0b01000;
/// x86 page fault flag. If set, the page fault was caused by an instruction
/// fetch.
pub const PAGE_FAULT_INSTRUCTION: u32 = 0b10000;

extern "C" {
	/// Enables paging with the given page directory.
	pub fn paging_enable(directory: *const u32);
	/// Disables paging.
	pub fn paging_disable();

	/// Executes the `invlpg` instruction for the address `addr`.
	fn invlpg(addr: *const c_void);
	/// Reloads the TLB (Translation Lookaside Buffer).
	pub fn tlb_reload();
}

/// When editing a virtual memory context, the kernel might edit pages in kernel
/// space.
///
/// These pages being shared with every contexts, another context might be modifying the same
/// context at the same time.
///
/// To prevent this issue, this mutex has to be locked whenever modifying kernel
/// space mappings.
static GLOBAL_MUTEX: Mutex<()> = Mutex::new(());

/// Tells whether the kernel tables are initialized.
static mut KERNEL_TABLES_INIT: bool = false;
/// Array storing kernel space paging tables.
static mut KERNEL_TABLES: [*mut u32; 256] = [0 as _; 256];

/// Returns the array of kernel space paging tables.
///
/// If the table is not initialized, the function initializes it.
///
/// The first time this function is called, it is **not** thread safe.
unsafe fn get_kernel_tables() -> Result<&'static [*mut u32; 256], Errno> {
	if !KERNEL_TABLES_INIT {
		for table in &mut KERNEL_TABLES {
			*table = alloc_obj()?;
		}

		KERNEL_TABLES_INIT = true;
	}

	Ok(&KERNEL_TABLES)
}

/// Returns the physical address to the `n`th kernel space paging table.
///
/// # Safety
///
/// The first time this function is called, it is **not** thread safe.
unsafe fn get_kernel_table(n: usize) -> Result<*mut u32, Errno> {
	let tables = get_kernel_tables()?;
	debug_assert!(n < tables.len());

	Ok(memory::kern_to_phys(tables[n] as _) as _)
}

/// Allocates a paging object and returns its virtual address.
///
/// If the allocation fails, the function returns an error.
fn alloc_obj() -> Result<*mut u32, Errno> {
	let ptr = buddy::alloc_kernel(0)? as *mut u8;

	// Zero memory
	let slice = unsafe { slice::from_raw_parts_mut(ptr, buddy::get_frame_size(0)) };
	slice.fill(0);

	Ok(ptr as _)
}

/// Returns the object at index `index` of given object `obj`.
fn obj_get(obj: *const u32, index: usize) -> u32 {
	debug_assert!(index < 1024);

	unsafe { ptr::read(obj_get_ptr(obj, index)) }
}

/// Sets the object at index `index` of given object `obj` with value `value`.
fn obj_set(obj: *mut u32, index: usize, value: u32) {
	debug_assert!(index < 1024);

	unsafe {
		ptr::write(obj_get_mut_ptr(obj, index), value);
	}
}

/// Returns a pointer to the object at index `index` of given object `obj`.
fn obj_get_ptr(obj: *const u32, index: usize) -> *const u32 {
	debug_assert!(index < 1024);
	let mut obj_ptr = obj;
	if (obj_ptr as *const c_void) < memory::PROCESS_END {
		obj_ptr = ((obj as usize) + (memory::PROCESS_END as usize)) as _;
	}

	unsafe { obj_ptr.add(index) }
}

/// Returns a mutable pointer to the object at index `index` of given object
/// `obj`.
fn obj_get_mut_ptr(obj: *mut u32, index: usize) -> *mut u32 {
	debug_assert!(index < 1024);
	let mut obj_ptr = obj;
	if (obj_ptr as *const c_void) < memory::PROCESS_END {
		obj_ptr = ((obj as usize) + (memory::PROCESS_END as usize)) as _;
	}

	unsafe { obj_ptr.add(index) }
}

/// Frees paging object `obj`. The pointer to the object must be a virtual
/// address.
fn free_obj(obj: *mut u32) {
	buddy::free_kernel(obj as _, 0)
}

/// The structure representing virtual memory context handler for the x86
/// architecture.
#[derive(Debug)]
pub struct X86VMem {
	/// The virtual address to the page directory.
	page_dir: *mut u32,
}

/// This module handles page tables manipulations.
mod table {
	use super::*;

	/// Creates an empty page table at index `index` of the page directory.
	pub fn create(vmem: *mut u32, index: usize, flags: u32) -> Result<(), Errno> {
		debug_assert!(index < 1024);
		debug_assert!(flags & ADDR_MASK == 0);
		debug_assert!(flags & FLAG_PAGE_SIZE == 0);

		let v = {
			if index < 768 {
				alloc_obj()?
			} else {
				// Safe because only one thread is running the first time this function is
				// called
				unsafe { get_kernel_table(index - 768)? }
			}
		};

		obj_set(
			vmem,
			index,
			(memory::kern_to_phys(v as _) as u32) | (flags | FLAG_PRESENT),
		);
		Ok(())
	}

	/// Expands a large block into a page table.
	///
	/// This function allocates a new page table and fills it so that the memory mapping keeps the
	/// same behavior.
	pub fn expand(vmem: *mut u32, index: usize) -> Result<(), Errno> {
		let mut dir_entry_value = obj_get(vmem, index);
		debug_assert!(dir_entry_value & FLAG_PRESENT != 0);
		debug_assert!(dir_entry_value & FLAG_PAGE_SIZE != 0);

		let base_addr = dir_entry_value & ADDR_MASK;
		let flags = dir_entry_value & FLAGS_MASK & !FLAG_PAGE_SIZE;
		table::create(vmem, index, flags)?;
		dir_entry_value = obj_get(vmem, index);
		let table_addr = (dir_entry_value & ADDR_MASK) as *mut u32;
		for i in 0..1024 {
			let addr = base_addr + (i * memory::PAGE_SIZE) as u32;
			obj_set(table_addr, i, addr | flags);
		}

		Ok(())
	}

	// TODO Use a counter instead. Increment it when mapping a page in the table and
	// decrement it when unmapping. Then return `true` if the counter has the value
	// `0`
	/// Tells whether the table at index `index` in the page directory is empty.
	pub fn is_empty(vmem: *mut u32, index: usize) -> bool {
		debug_assert!(index < 1024);

		let dir_entry_value = obj_get(vmem, index);
		let dir_entry_addr = (dir_entry_value & ADDR_MASK) as *mut u32;

		for i in 0..1024 {
			if obj_get(dir_entry_addr, i) & FLAG_PRESENT != 0 {
				return false;
			}
		}

		true
	}

	/// Deletes the table at index `index` in the page directory.
	pub fn delete(vmem: *mut u32, index: usize) {
		debug_assert!(index < 1024);
		let dir_entry_value = obj_get(vmem, index);
		let dir_entry_addr = (dir_entry_value & ADDR_MASK) as *const u32;
		free_obj(memory::kern_to_virt(dir_entry_addr as _) as _);
		obj_set(vmem, index, 0);
	}
}

impl X86VMem {
	/// Asserts that the map operation shall not result in a crash.
	#[cfg(config_debug_debug)]
	fn check_map(&self, virt_ptr: *const c_void, phys_ptr: *const c_void, pse: bool) {
		if !self.is_bound() {
			return;
		}

		let esp = unsafe { crate::register_get!("esp") as *const c_void };
		let aligned_stack_page = crate::util::down_align(esp, memory::PAGE_SIZE);
		let size = {
			if pse {
				1024
			} else {
				1
			}
		};
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

		let esp = unsafe { crate::register_get!("esp") as *const c_void };
		let aligned_stack_page = crate::util::down_align(esp, memory::PAGE_SIZE);
		let size = {
			if pse {
				1024
			} else {
				1
			}
		};

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
	pub fn new() -> Result<Self, Errno> {
		let vmem = Self {
			page_dir: alloc_obj()?,
		};

		let flags = FLAG_PRESENT | FLAG_WRITE | FLAG_USER | FLAG_GLOBAL;
		for i in 0..256 {
			// Safe because only one thread is running when the first vmem is created
			let ptr = unsafe { get_kernel_table(i)? };

			obj_set(vmem.page_dir, 768 + i, ptr as u32 | flags);
		}

		Ok(vmem)
	}

	/// Returns the index of the element corresponding to the given virtual
	/// address `ptr` for element at level `level` in the tree.
	///
	/// The level represents the depth in the tree. `0` is the deepest.
	fn get_addr_element_index(ptr: *const c_void, level: usize) -> usize {
		((ptr as usize) >> (12 + level * 10)) & 0x3ff
	}

	// TODO Adapt to 5 level paging
	/// Resolves the paging entry for the given pointer.
	///
	/// If no entry is found, `None` is returned.
	///
	/// The entry must be marked as present to be found.
	///
	/// If Page Size Extension (PSE) is used, an entry of the page directory might
	/// be returned.
	pub fn resolve(&self, ptr: *const c_void) -> Option<*const u32> {
		let dir_entry_index = Self::get_addr_element_index(ptr, 1);
		let dir_entry_value = obj_get(self.page_dir, dir_entry_index);
		if dir_entry_value & FLAG_PRESENT == 0 {
			return None;
		}
		if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			return Some(obj_get_ptr(self.page_dir, dir_entry_index));
		}

		let table = memory::kern_to_virt((dir_entry_value & ADDR_MASK) as *const u32);
		let table_entry_index = Self::get_addr_element_index(ptr, 0);
		let table_entry_value = obj_get(table, table_entry_index);
		if table_entry_value & FLAG_PRESENT == 0 {
			return None;
		}
		Some(obj_get_ptr(table, table_entry_index))
	}

	/// Resolves the entry for the given virtual address `ptr` and returns its
	/// flags.
	///
	/// This function might return a page directory entry if a large
	/// block is present at the corresponding location.
	///
	/// If no entry is found, the function returns `None`.
	pub fn get_flags(&self, ptr: *const c_void) -> Option<u32> {
		self.resolve(ptr).map(|e| unsafe { *e & FLAGS_MASK })
	}

	/// Tells whether to use PSE mapping for the given virtual address `addr`
	/// and remaining pages `pages`.
	fn use_pse(addr: *const c_void, pages: usize) -> bool {
		// The end address of the hypothetical PSE block
		let pse_end = (addr as usize).wrapping_add(1024 * memory::PAGE_SIZE);

		// Ensuring no PSE block is created in kernel space
		pse_end < (memory::PROCESS_END as usize)
		// Ensuring the virtual address doesn't overflow
			&& pse_end >= (addr as usize)
		// Checking the address is aligned on the PSE boundary
			&& addr.is_aligned_to(1024 * memory::PAGE_SIZE)
		// Checking that there remain enough pages to make a PSE block
			&& pages >= 1024
	}

	/// Maps the given physical address `physaddr` to the given virtual address
	/// `virtaddr` with the given flags using blocks of 1024 pages (PSE).
	fn map_pse(&mut self, physaddr: *const c_void, virtaddr: *const c_void, mut flags: u32) {
		debug_assert!(physaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!(flags & ADDR_MASK == 0);

		flags |= FLAG_PRESENT | FLAG_PAGE_SIZE;

		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry_value = obj_get(self.page_dir, dir_entry_index);
		if dir_entry_index < 768
			&& dir_entry_value & FLAG_PRESENT != 0
			&& dir_entry_value & FLAG_PAGE_SIZE == 0
		{
			table::delete(self.page_dir, dir_entry_index);
		}

		obj_set(self.page_dir, dir_entry_index, (physaddr as u32) | flags);
	}

	/// Unmaps the large block (PSE) at the given virtual address `virtaddr`.
	fn unmap_pse(&mut self, virtaddr: *const c_void) {
		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry_value = obj_get(self.page_dir, dir_entry_index);
		if dir_entry_value & FLAG_PRESENT == 0 || dir_entry_value & FLAG_PAGE_SIZE == 0 {
			return;
		}

		obj_set(self.page_dir, dir_entry_index, 0);
	}
}

impl VMem for X86VMem {
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void> {
		if let Some(e) = self.resolve(ptr) {
			let entry_value = unsafe { *e };
			let remain_mask = if entry_value & FLAG_PAGE_SIZE == 0 {
				memory::PAGE_SIZE - 1
			} else {
				1024 * memory::PAGE_SIZE - 1
			};

			let mut virtptr = (entry_value & ADDR_MASK) as usize;
			virtptr |= ptr as usize & remain_mask;
			Some(virtptr as _)
		} else {
			None
		}
	}

	fn map(
		&mut self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		mut flags: u32,
	) -> Result<(), Errno> {
		#[cfg(config_debug_debug)]
		self.check_map(virtaddr, physaddr, false);

		debug_assert!(physaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert_eq!(flags & ADDR_MASK, 0);

		flags |= FLAG_PRESENT;

		// Locking the global mutex to avoid data races while modifying kernel space
		// tables
		let _ = GLOBAL_MUTEX.lock();

		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let mut dir_entry_value = obj_get(self.page_dir, dir_entry_index);
		if dir_entry_value & FLAG_PRESENT == 0 {
			table::create(self.page_dir, dir_entry_index, flags)?;
		} else if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, dir_entry_index)?;
		}
		dir_entry_value = obj_get(self.page_dir, dir_entry_index);

		if dir_entry_index < 768 {
			// Setting the table's flags
			dir_entry_value |= flags;
			obj_set(self.page_dir, dir_entry_index, dir_entry_value);
		}

		debug_assert!(dir_entry_value & FLAG_PAGE_SIZE == 0);
		let table = (dir_entry_value & ADDR_MASK) as *mut u32;
		let table_entry_index = Self::get_addr_element_index(virtaddr, 0);
		obj_set(table, table_entry_index, (physaddr as u32) | flags);

		// Invalidating the page
		self.invalidate_page(virtaddr);

		Ok(())
	}

	fn map_range(
		&mut self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		pages: usize,
		flags: u32,
	) -> Result<(), Errno> {
		debug_assert!(physaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!(
			(virtaddr as usize / memory::PAGE_SIZE) + pages
				<= (usize::MAX / memory::PAGE_SIZE) + 1
		);
		debug_assert_eq!(flags & ADDR_MASK, 0);

		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let next_physaddr = ((physaddr as usize) + off) as *const c_void;
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;

			if Self::use_pse(next_virtaddr, pages - i) {
				#[cfg(config_debug_debug)]
				self.check_map(next_virtaddr, next_physaddr, true);

				self.map_pse(next_physaddr, next_virtaddr, flags);
				i += 1024;

				// Invalidating the pages
				self.invalidate_page(next_virtaddr); // TODO Check if invalidating the whole
				                     // table
			} else {
				self.map(next_physaddr, next_virtaddr, flags)?;
				i += 1;
			}
		}

		Ok(())
	}

	fn unmap(&mut self, virtaddr: *const c_void) -> Result<(), Errno> {
		#[cfg(config_debug_debug)]
		self.check_unmap(virtaddr, false);

		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));

		// Locking the global mutex to avoid data races while modifying kernel space
		// tables
		let _ = GLOBAL_MUTEX.lock();

		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry_value = obj_get(self.page_dir, dir_entry_index);
		if dir_entry_value & FLAG_PRESENT == 0 {
			return Ok(());
		} else if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, dir_entry_index)?;
		}

		let table = (dir_entry_value & ADDR_MASK) as *mut u32;
		let table_entry_index = Self::get_addr_element_index(virtaddr, 0);
		obj_set(table, table_entry_index, 0);

		// Invalidating the page
		self.invalidate_page(virtaddr);

		// Removing the table if it is empty and if not a kernel space table
		if table::is_empty(self.page_dir, dir_entry_index) && dir_entry_index < 768 {
			table::delete(self.page_dir, dir_entry_index);
		}

		Ok(())
	}

	fn unmap_range(&mut self, virtaddr: *const c_void, pages: usize) -> Result<(), Errno> {
		debug_assert!(virtaddr.is_aligned_to(memory::PAGE_SIZE));
		debug_assert!((virtaddr as usize) + (pages * memory::PAGE_SIZE) >= (virtaddr as usize));

		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;

			// Checking whether the page is mapped in PSE
			let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
			let dir_entry_value = obj_get(self.page_dir, dir_entry_index);
			let is_pse =
				(dir_entry_value & FLAG_PAGE_SIZE != 0) && Self::use_pse(next_virtaddr, pages - i);

			if is_pse {
				self.unmap_pse(next_virtaddr);
				i += 1024;

				// Invalidating the pages
				self.invalidate_page(next_virtaddr); // TODO Check if invalidating the whole
				                     // table
			} else {
				self.unmap(next_virtaddr)?;
				i += 1;
			}
		}

		Ok(())
	}

	fn bind(&self) {
		if !self.is_bound() {
			unsafe {
				paging_enable(memory::kern_to_phys(self.page_dir as _) as _);
			}
		}
	}

	fn is_bound(&self) -> bool {
		unsafe { cpu::cr3_get() == memory::kern_to_phys(self.page_dir as _) as _ }
	}

	fn invalidate_page(&self, addr: *const c_void) {
		// TODO Also invalidate on other CPU core (TLB shootdown)

		unsafe {
			invlpg(addr);
		}
	}

	fn flush(&self) {
		// TODO Also invalidate on other CPU core (TLB shootdown)

		if self.is_bound() {
			unsafe {
				tlb_reload();
			}
		}
	}
}

impl TryClone for X86VMem {
	fn try_clone(&self) -> Result<Self, Errno> {
		let s = Self {
			page_dir: alloc_obj()?,
		};

		for i in 0..1024 {
			let src_dir_entry_value = obj_get(self.page_dir, i);
			if src_dir_entry_value & FLAG_PRESENT == 0 {
				continue;
			}

			if src_dir_entry_value & FLAG_PAGE_SIZE == 0 {
				let src_table = (src_dir_entry_value & ADDR_MASK) as *const u32;
				let src_table = memory::kern_to_virt(src_table as _) as _;

				let dest_table = {
					if i < 768 {
						let dest_table = alloc_obj()?;

						unsafe {
							// Safe because pointers are valid
							ptr::copy_nonoverlapping::<u32>(src_table, dest_table, 1024);
						}
						dest_table
					} else {
						// Safe because only one thread is running the first time this function is
						// called
						unsafe { get_kernel_table(i - 768)? }
					}
				};

				obj_set(
					s.page_dir,
					i,
					(memory::kern_to_phys(dest_table as _) as u32)
						| (src_dir_entry_value & FLAGS_MASK),
				);
			} else {
				obj_set(s.page_dir, i, src_dir_entry_value);
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
			crate::kernel_panic!("Dropping virtual memory context handler while in use!");
		}

		for i in 0..768 {
			let dir_entry_value = obj_get(self.page_dir, i);

			if (dir_entry_value & FLAG_PRESENT) != 0 && (dir_entry_value & FLAG_PAGE_SIZE) == 0 {
				let table = (dir_entry_value & ADDR_MASK) as *mut u32;
				free_obj(memory::kern_to_virt(table as _) as _);
			}
		}

		free_obj(self.page_dir as _);
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::vga;

	#[test_case]
	fn vmem_x86_vga_text_access() {
		let vmem = X86VMem::new().unwrap();
		for i in 0..(80 * 25 * 2) {
			assert!(vmem.translate(((vga::get_buffer_virt() as usize) + i) as _) != None);
		}
	}
}
