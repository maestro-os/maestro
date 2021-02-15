/// x86 virtual memory works with a tree structure. Each element is an array of subelements. The
/// position of the elements in the arrays allows to tell the virtual address for the mapping.
/// Under 32 bits, elements are array of 32 bits long words that can contain 1024 entries. The
/// following elements are available:
/// - Page directory: The main element, contains page tables
/// - Page table: Represents a block of 4MB, each entry is a page
///
/// Under 32 bits, pages are 4096 bytes large. Each entries of elements contains the physical
/// address to the element/page and some flags. The flags can be stored with the address in only
/// 4 bytes large entries because addresses have to be page-aligned, freeing 12 bits in the entry
/// for the flags.
///
/// For each entries of each elements, the kernel must keep track of how many elements are being
/// used. This can be done with a simple counter: when an entry is allocated, the counter is
/// incremented and when an entry is freed, the counter is decremented. When the counter reaches 0,
/// the element can be freed.
///
/// The Page Size Extension (PSE) allows to map 4MB large blocks without using a page table.

use core::ffi::c_void;
use core::result::Result;
use crate::elf;
use crate::memory::NULL;
use crate::memory::buddy;
use crate::memory::vmem::VMem;
use crate::memory;
use crate::multiboot;
use crate::util;
use crate::vga;

/// x86 paging flag. If set, prevents the CPU from updating the associated addresses when the TLB
/// is flushed.
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
/// Address mask in a page directory entry. The address doesn't need every bytes since it must be
/// page-aligned.
pub const ADDR_MASK: u32 = !FLAGS_MASK;

/// x86 page fault flag. If set, the page was present.
pub const PAGE_FAULT_PRESENT: u32 = 0b00001;
/// x86 page fault flag. If set, the error was caused by a write operation, else the error was
/// caused by a read operation.
pub const PAGE_FAULT_WRITE: u32 = 0b00010;
/// x86 page fault flag. If set, the page fault was caused by a userspace operation.
pub const PAGE_FAULT_USER: u32 = 0b00100;
/// x86 page fault flag. If set, one or more page directory entries contain reserved bits which are
/// set.
pub const PAGE_FAULT_RESERVED: u32 = 0b01000;
/// x86 page fault flag. If set, the page fault was caused by an instruction fetch.
pub const PAGE_FAULT_INSTRUCTION: u32 = 0b10000;

/// Structure wrapping a virtual memory. This structure contains the counter for the number of
/// elements that are used in the associated element.
pub struct VMemWrapper {
	/// The number of used elements in the associated element
	used_elements: u16,
	/// The associated element
	vmem: *mut u32,
}

// TODO Find a place to store wrappers

extern "C" {
	pub fn cr0_get() -> u32;
	pub fn cr0_set(flags: u32);
	pub fn cr0_clear(flags: u32);
	pub fn cr2_get() -> u32;
	pub fn cr3_get() -> u32;

	pub fn paging_enable(directory: *const u32);
	pub fn paging_disable();
	pub fn tlb_reload();
}

/// Allocates a paging object and returns its virtual address.
/// Returns Err if the allocation fails.
fn alloc_obj() -> Result<*mut u32, ()> {
	let ptr = buddy::alloc_kernel(0)? as *mut c_void;
	unsafe {
		util::bzero(ptr as _, buddy::get_frame_size(0));
	}
	Ok(ptr as _)
}

/// Frees paging object `obj`. The pointer to the object must be a virtual address.
fn free_obj(obj: *mut u32) {
	buddy::free_kernel(obj as _, 0)
}

/// The structure representing virtual memory context handler for the x86 architecture.
pub struct X86VMem {
	/// The virtual address to the page directory.
	page_dir: *mut u32,
}

/// This module handles page tables manipulations.
mod table {
	use super::*;

	/// Creates an empty page table at index `index` of the page directory.
	pub fn create(vmem: *mut u32, index: usize, flags: u32) -> Result<(), ()> {
		debug_assert!(index < 1024);
		debug_assert!(flags & ADDR_MASK == 0);
		debug_assert!(flags & FLAG_PAGE_SIZE == 0);

		let v = alloc_obj()?;
		unsafe {
			*vmem.add(index) = (memory::kern_to_phys(v as _) as u32) | (flags | FLAG_PRESENT);
		}
		Ok(())
	}

	/// Expands a large block into a page table. This function allocates a new page table and fills
	/// it so that the memory mapping keeps the same behavior.
	pub fn expand(vmem: *mut u32, index: usize) -> Result<(), ()> {
		let dir_entry = unsafe { vmem.add(index) };
		let mut dir_entry_value = unsafe { *dir_entry };
		debug_assert!(dir_entry_value & FLAG_PRESENT != 0);
		debug_assert!(dir_entry_value & FLAG_PAGE_SIZE != 0);

		let base_addr = dir_entry_value & ADDR_MASK;
		let flags = dir_entry_value & FLAGS_MASK & !FLAG_PAGE_SIZE;
		table::create(vmem, index, flags)?;
		dir_entry_value = unsafe { *dir_entry };
		let table_addr = (dir_entry_value & ADDR_MASK) as *mut u32;
		for i in 0..1024 {
			let addr = base_addr + (i * memory::PAGE_SIZE) as u32;
			unsafe {
				*table_addr.add(i) = addr | flags;
			}
		}

		Ok(())
	}

	/// Deletes the table at index `index` in the page directory.
	pub fn delete(vmem: *mut u32, index: usize) {
		debug_assert!(index < 1024);
		let dir_entry = unsafe { vmem.add(index) };
		let dir_entry_value = unsafe { *dir_entry };
		let dir_entry_addr = dir_entry_value & ADDR_MASK;
		buddy::free(dir_entry_addr as _, 0);
		unsafe {
			*dir_entry = 0;
		}
	}
}

impl X86VMem {
	/// Protects the kernel's read-only sections from writing in the given page directory `vmem`.
	fn protect_kernel(&mut self) {
		let boot_info = multiboot::get_boot_info();
		elf::foreach_sections(boot_info.elf_sections, boot_info.elf_num as usize,
			boot_info.elf_shndx as usize, boot_info.elf_entsize as usize,
			| section: &elf::ELF32SectionHeader, _name: &str | {
				if section.sh_flags & elf::SHF_WRITE != 0
					|| section.sh_addralign as usize != memory::PAGE_SIZE {
					return;
				}

				let phys_addr = if section.sh_addr < (memory::PROCESS_END as _) {
					section.sh_addr as *const c_void
				} else {
					memory::kern_to_phys(section.sh_addr as _)
				};
				let virt_addr = if section.sh_addr >= (memory::PROCESS_END as _) {
					section.sh_addr as *const c_void
				} else {
					memory::kern_to_virt(section.sh_addr as _)
				};
				let pages = util::ceil_division(section.sh_size, memory::PAGE_SIZE as _) as usize;
				if self.map_range(phys_addr, virt_addr, pages as usize, FLAG_USER).is_err() {
					crate::kernel_panic!("Kernel protection failed!");
				}
			});
	}

	/// Initializes a new page directory. The kernel memory is mapped into the context by default.
	pub fn new() -> Result<Self, ()> {
		let mut vmem = Self {
			page_dir: alloc_obj()?,
		};
		vmem.identity(NULL, 0)?;
		// TODO If Meltdown mitigation is enabled, only allow read access to a stub for interrupts
		// TODO Place pages count in a constant
		vmem.map_range(NULL, memory::PROCESS_END, 262144, FLAG_WRITE)?;
		// TODO Extend to other DMA
		vmem.map_range(vga::BUFFER_PHYS as _, vga::BUFFER_VIRT as _, 1,
			FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH)?;
		vmem.protect_kernel();
		Ok(vmem)
	}

	/// Returns the index of the element corresponding to the given virtual address `ptr` for
	/// element at level `level` in the tree. The level represents the depth in the tree. `0` is
	/// the deepest.
	fn get_addr_element_index(ptr: *const c_void, level: usize) -> usize {
		((ptr as usize) >> (12 + level * 10)) & 0x3ff
	}

	// TODO Adapt to 5 level paging
	/// Resolves the paging entry for the given pointer. If no entry is found, None is returned.
	/// The entry must be marked as present to be found. If Page Size Extension (PSE) is used, an
	/// entry of the page directory might be returned.
	pub fn resolve(&self, ptr: *const c_void) -> Option<*const u32> {
		let dir_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(Self::get_addr_element_index(ptr, 1))
		};
		let dir_entry_value = unsafe { *dir_entry };
		if dir_entry_value & FLAG_PRESENT == 0 {
			return None;
		}
		if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			return Some(dir_entry);
		}

		let table = memory::kern_to_virt((dir_entry_value & ADDR_MASK) as _) as *const u32;
		let table_entry = unsafe { // Pointer arithmetic
			table.add(Self::get_addr_element_index(ptr, 0))
		};
		let table_entry_value = unsafe { *table_entry };
		if table_entry_value & FLAG_PRESENT == 0 {
			// TODO
			return None;
		}
		Some(table_entry)
	}

	/// Resolves the entry for the given virtual address `ptr` and returns its flags. This function
	/// might return a page directory entry if a large block is present at the corresponding
	/// location. If no entry is found, the function returns None.
	pub fn get_flags(&self, ptr: *const c_void) -> Option<u32> {
		if let Some(e) = self.resolve(ptr) {
			Some(unsafe { *e } & FLAGS_MASK)
		} else {
			None
		}
	}

	/// Maps the given physical address `physaddr` to the given virtual address `virtaddr` with the
	/// given flags using blocks of 1024 pages (PSE).
	pub fn map_pse(&mut self, physaddr: *const c_void, virtaddr: *const c_void, flags: u32) {
		debug_assert!(util::is_aligned(physaddr, memory::PAGE_SIZE));
		debug_assert!(util::is_aligned(virtaddr, memory::PAGE_SIZE));
		debug_assert!(flags & ADDR_MASK == 0);

		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(dir_entry_index)
		};
		let dir_entry_value = unsafe { // Dereference of raw pointer
			*dir_entry
		};
		if dir_entry_value & FLAG_PRESENT != 0
			&& dir_entry_value & FLAG_PAGE_SIZE == 0 {
			table::delete(self.page_dir, dir_entry_index);
		}

		unsafe { // Pointer arithmetic and dereference of raw pointer
			*self.page_dir.add(dir_entry_index) = (physaddr as u32)
				| (flags | FLAG_PRESENT | FLAG_PAGE_SIZE);
		}
	}

	/// Maps the physical address `ptr` to the same address in virtual memory with the given flags
	/// `flags`, using blocks of 1024 pages (PSE).
	pub fn identity_pse(&mut self, ptr: *const c_void, flags: u32) {
		self.map_pse(ptr, ptr, flags);
	}

	/// Unmaps the large block (PSE) at the given virtual address `virtaddr`.
	pub fn unmap_pse(&mut self, virtaddr: *const c_void) {
		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(dir_entry_index) as *mut u32
		};
		let dir_entry_value = unsafe { // Dereference of raw pointer
			*dir_entry
		};
		if dir_entry_value & FLAG_PRESENT == 0
			|| dir_entry_value & FLAG_PAGE_SIZE == 0 {
			return;
		}
		unsafe {
			*dir_entry = 0;
		}
	}
}

impl VMem for X86VMem {
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void> {
		if let Some(e) = self.resolve(ptr) {
			Some((unsafe { // Dereference of raw pointer
				*e
			} & ADDR_MASK) as _) // TODO Add remaining offset (check if PSE is used)
		} else {
			None
		}
	}

	fn map(&mut self, physaddr: *const c_void, virtaddr: *const c_void, flags: u32)
		-> Result<(), ()> {
		debug_assert!(util::is_aligned(physaddr, memory::PAGE_SIZE));
		debug_assert!(util::is_aligned(virtaddr, memory::PAGE_SIZE));
		debug_assert!(flags & ADDR_MASK == 0);

		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(dir_entry_index)
		};
		let mut dir_entry_value = unsafe { // Dereference of raw pointer
			*dir_entry
		};
		if dir_entry_value & FLAG_PRESENT == 0 {
			table::create(self.page_dir, dir_entry_index, flags)?;
		} else if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, dir_entry_index)?;
		}

		dir_entry_value = unsafe { // Dereference of raw pointer
			*dir_entry
		};
		debug_assert!(dir_entry_value & FLAG_PAGE_SIZE == 0);
		let table = (dir_entry_value & ADDR_MASK) as *mut u32;
		let table_entry_index = Self::get_addr_element_index(virtaddr, 0);
		let table_entry = unsafe { // Pointer arithmetic
			table.add(table_entry_index)
		};
		unsafe {
			*table_entry = (physaddr as u32) | (flags | FLAG_PRESENT);
		}

		Ok(())
	}

	fn map_range(&mut self, physaddr: *const c_void, virtaddr: *const c_void, pages: usize,
		flags: u32) -> Result<(), ()> {
		debug_assert!(util::is_aligned(physaddr, memory::PAGE_SIZE));
		debug_assert!(util::is_aligned(virtaddr, memory::PAGE_SIZE));
		debug_assert!(flags & ADDR_MASK == 0);

		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let use_pse = {
				util::is_aligned(((virtaddr as usize) + off) as _, 1024 * memory::PAGE_SIZE)
					&& (pages - i) >= 1024
			};
			let next_physaddr = ((physaddr as usize) + off) as *const c_void;
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;
			if use_pse {
				self.map_pse(next_physaddr, next_virtaddr, flags);
				i += 1024;
			} else {
				if self.map(next_physaddr, next_virtaddr, flags) == Err(()) {
					// TODO Undo
				}
				i += 1;
			}
		}

		Ok(())
	}

	fn unmap(&mut self, virtaddr: *const c_void) -> Result<(), ()> {
		let dir_entry_index = Self::get_addr_element_index(virtaddr, 1);
		let dir_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(dir_entry_index) as *const u32
		};
		let dir_entry_value = unsafe { // Dereference of raw pointer
			*dir_entry
		};
		if dir_entry_value & FLAG_PRESENT == 0 {
			return Ok(());
		} else if dir_entry_value & FLAG_PAGE_SIZE != 0 {
			table::expand(self.page_dir, dir_entry_index)?;
		}

		let table_entry_index = Self::get_addr_element_index(virtaddr, 0);
		let table_entry = unsafe { // Pointer arithmetic
			self.page_dir.add(table_entry_index) as *mut u32
		};
		unsafe {
			*table_entry = 0;
		}
		Ok(())
	}

	fn unmap_range(&mut self, virtaddr: *const c_void, pages: usize) -> Result<(), ()> {
		debug_assert!(util::is_aligned(virtaddr, memory::PAGE_SIZE));
		debug_assert!((virtaddr as usize) + (pages * memory::PAGE_SIZE) >= (virtaddr as usize));

		let mut i = 0;
		while i < pages {
			let off = i * memory::PAGE_SIZE;
			let use_pse = {
				util::is_aligned(((virtaddr as usize) + off) as _, 1024 * memory::PAGE_SIZE)
					&& (pages - i) >= 1024
			};
			let next_virtaddr = ((virtaddr as usize) + off) as *const c_void;
			if use_pse {
				self.unmap_pse(next_virtaddr);
				i += 1024;
			} else {
				if self.unmap(next_virtaddr) == Err(()) {
					// TODO Undo
				}
				i += 1;
			}
		}

		Ok(())
	}

	fn clone(&self) -> Result::<Self, ()> {
		let v = alloc_obj()?;
		for i in 0..1024 {
			let src_dir_entry = unsafe { // Pointer arithmetic
				self.page_dir.add(i)
			};
			let src_dir_entry_value = unsafe { // Pointer arithmetic
				*src_dir_entry
			};
			if src_dir_entry_value & FLAG_PRESENT == 0 {
				continue;
			}

			let dest_dir_entry = unsafe {
				self.page_dir.add(i) as *mut u32
			};
			if src_dir_entry_value & FLAG_PAGE_SIZE == 0 {
				let src_table = (src_dir_entry_value & ADDR_MASK) as *const u32;
				if let Ok(dest_table) = alloc_obj() {
					unsafe {
						util::memcpy(dest_table as _, src_table as _, memory::PAGE_SIZE);
						*dest_dir_entry = (memory::kern_to_phys(dest_table as _) as u32)
							| (src_dir_entry_value & FLAGS_MASK);
					}
				} else {
					return Err(());
				}
			} else {
				unsafe {
					*dest_dir_entry = src_dir_entry_value;
				}
			}
		}

		Ok(Self {
			page_dir: v
		})
	}

	fn bind(&self) {
		unsafe { // Call to C function
			paging_enable(memory::kern_to_phys(self.page_dir as _) as _);
		}
	}

	fn is_bound(&self) -> bool {
		unsafe { // Call to C function
			self.page_dir == (cr3_get() as _)
		}
	}

	fn flush(&self) {
		if self.is_bound() {
			unsafe { // Call to C function
				tlb_reload();
			}
		}
	}
}

impl Drop for X86VMem {
	/// Destroyes the given page directory, including its children elements. If the page directory
	/// is begin used, the behaviour is undefined.
	fn drop(&mut self) {
		if self.is_bound() {
			crate::kernel_panic!("Dropping virtual memory context handler while in use!", 0);
		}

		for i in 0..1024 {
			let dir_entry = unsafe { // Pointer arithmetic
				self.page_dir.add(i)
			};
			let dir_entry_value = unsafe { // Dereference of raw pointer
				*dir_entry
			};
			if (dir_entry_value & FLAG_PRESENT) != 0 && (dir_entry_value & FLAG_PAGE_SIZE) == 0 {
				let table = (dir_entry_value & ADDR_MASK) as *mut u32;
				free_obj(memory::kern_to_virt(table as _) as _);
			}
		}

		free_obj(self.page_dir as _);
	}
}

// TODO Unit tests
