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

	/// Pointer to the beginning of the main block of physical allocatable memory, page aligned.
	pub phys_main_begin: *const c_void,
	/// The size of the main block of physical allocatable memory, in pages.
	pub phys_main_pages: usize,
}

/// Variable containing the memory mapping.
static mut MEM_INFO: MaybeUninit<MemoryInfo> = MaybeUninit::uninit();

/// Returns the structure storing memory mapping informations.
pub fn get_info() -> &'static MemoryInfo {
	unsafe {
		MEM_INFO.assume_init_mut()
	}
}

/// Prints the physical memory mapping.
pub fn print_entries() {
	let mem_info = get_info();
	debug_assert!(!mem_info.memory_maps.is_null());

	crate::println!("--- Memory mapping ---");
	crate::println!("<begin> <end> <type>");

	let mut ptr = mem_info.memory_maps;
	while (ptr as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size as usize) {
		let entry = unsafe {
			&*ptr
		};

		if entry.is_valid() {
			let begin = entry.addr;
			let end = begin + entry.len;
			let type_ = entry.get_type_string();

			crate::println!("- 0x{:x} 0x{:x} {}", begin, end, type_);
		}

		ptr = ((ptr as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
}

/// Returns the pointer to the beginning of the main physical allocatable memory and its size in
/// number of pages.
fn get_phys_main(multiboot_ptr: *const c_void) -> (*const c_void, usize) {
	let boot_info = multiboot::get_boot_info();

	// The end of the kernel code
	let mut begin = memory::get_kernel_end();

	let multiboot_tags_size = multiboot::get_tags_size(multiboot_ptr);
	// The end of multiboot tags
	let multiboot_tags_end = ((multiboot_ptr as usize) + multiboot_tags_size) as *const _;
	begin = max(begin, multiboot_tags_end);

	// The end of ELF sections list
	let elf_sections_end = (boot_info.elf_sections as usize + boot_info.elf_num as usize
		* boot_info.elf_entsize as usize) as *const c_void;
	begin = max(begin, elf_sections_end);

	// The end of the ELF sections' content
	let elf_end = elf::get_sections_end(boot_info.elf_sections, boot_info.elf_num as _,
		boot_info.elf_entsize as _);
	begin = max(begin, elf_end);

	// Page-align
	begin = util::align(begin, memory::PAGE_SIZE);

	// TODO Handle 64-bits systems
	let pages = min(boot_info.mem_upper / 4, 1048576) as usize
		- ((begin as usize) / memory::PAGE_SIZE);
	(begin, pages)
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

	let (main_begin, main_pages) = get_phys_main(multiboot_ptr);
	mem_info.phys_main_begin = main_begin;
	mem_info.phys_main_pages = main_pages;
}
