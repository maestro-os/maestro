//! This module handles the memory informations, which stores global informations on the system
//! memory by retrieving them from the boot informations. These data are meant to be used by the
//! memory allocators.

use core::cmp::*;
use core::mem::MaybeUninit;
use crate::elf;
use crate::memory::*;
use crate::memory;
use crate::multiboot;
use crate::util;

/// Structure storing informations relative to the main memory.
pub struct MemoryInfo {
	/// Size of the Multiboot2 memory map
	pub memory_maps_size: usize,
	/// Size of an entry in the Multiboot2 memory map
	pub memory_maps_entry_size: usize,
	/// Pointer to the Multiboot2 memory map
	pub memory_maps: *const multiboot::MmapEntry,

	/// Pointer to the end of the physical memory
	pub memory_end: *const c_void,
	/// Pointer to the beginning of physical allocatable memory, page aligned
	pub phys_alloc_begin: *const c_void,
	/// Pointer to the end of physical allocatable memory, page aligned
	pub phys_alloc_end: *const c_void,
	/// The total amount of allocatable memory in bytes
	pub available_memory: usize,
}

/// Variable containing the memory mapping.
static mut MEM_INFO: MaybeUninit<MemoryInfo> = MaybeUninit::uninit();

/// Returns the structure storing memory mapping informations.
pub fn get_info() -> &'static MemoryInfo {
	unsafe {
		MEM_INFO.assume_init_mut()
	}
}

/// Prints the memory mapping.
pub fn print_entries() {
	let mem_info = get_info();
	debug_assert!(!mem_info.memory_maps.is_null());

	crate::println!("--- Memory mapping ---");
	crate::println!("<begin> <end> <type>");

	let mut t = mem_info.memory_maps;
	while (t as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size as usize) {
		unsafe {
			if (*t).is_valid() {
				let begin = (*t).addr as *const c_void;
				let end = (((*t).addr as usize) + ((*t).len as usize)) as *const c_void;
				let type_ = (*t).get_type_string();
				crate::println!("- {:p} {:p} {}", begin, end, type_);
			}
		}
		t = ((t as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
}

/// Returns a pointer to the beginning of the allocatable physical memory.
fn get_phys_alloc_begin(multiboot_ptr: *const c_void) -> *const c_void {
	let boot_info = multiboot::get_boot_info();

	// The end of the kernel code
	let mut ptr = memory::get_kernel_end();

	let multiboot_tags_size = multiboot::get_tags_size(multiboot_ptr);
	// The end of multiboot tags
	let multiboot_tags_end = ((multiboot_ptr as usize) + multiboot_tags_size) as *const _;
	ptr = max(ptr, multiboot_tags_end);

	// The end of ELF sections list
	let elf_sections_end = (boot_info.elf_sections as usize + boot_info.elf_num as usize
		* boot_info.elf_entsize as usize) as *const c_void;
	ptr = max(ptr, elf_sections_end);

	// The end of the ELF sections' content
	let elf_end = elf::get_sections_end(boot_info.elf_sections, boot_info.elf_num as _,
		boot_info.elf_entsize as _);
	ptr = max(ptr, elf_end);

	util::align(ptr, memory::PAGE_SIZE)
}

/// Returns a pointer to the end of the system memory.
fn get_memory_end() -> *const c_void {
	let mem_info = get_info();
	debug_assert!(!mem_info.memory_maps.is_null());

	let mut t = mem_info.memory_maps;
	let mut end: usize = 0;

	while (t as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size as usize) {
		unsafe {
			if (*t).is_valid() {
				end = max(end, ((*t).addr as usize) + ((*t).len as usize));
			}
		}
		t = ((t as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
	end as *const _
}

/// Fills the memory mapping structure according to Multiboot's informations.
pub fn init(multiboot_ptr: *const c_void) {
	let boot_info = multiboot::get_boot_info();
	let mem_info = unsafe {
		MEM_INFO.assume_init_mut()
	};
	mem_info.memory_maps_size = boot_info.memory_maps_size;
	mem_info.memory_maps_entry_size = boot_info.memory_maps_entry_size;
	mem_info.memory_maps = boot_info.memory_maps;
	mem_info.memory_end = get_memory_end();
	mem_info.phys_alloc_begin = get_phys_alloc_begin(multiboot_ptr);
	mem_info.phys_alloc_end = util::down_align((boot_info.mem_upper * 1024) as *const _,
		memory::PAGE_SIZE);
	debug_assert!(mem_info.phys_alloc_begin < mem_info.phys_alloc_end);
	mem_info.available_memory = (mem_info.phys_alloc_end as usize)
		- (mem_info.phys_alloc_begin as usize);
}
