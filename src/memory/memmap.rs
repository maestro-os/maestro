/*
 * TODO doc
 */

use core::cmp::*;
use crate::memory::Void;
use crate::memory;
use crate::multiboot;
use crate::panic;
use crate::util;

/*
 * Structure storing informations relative to the main memory.
 */
pub struct MemoryInfo {
	/* Size of the Multiboot2 memory map */
	memory_maps_size: usize,
	/* Size of an entry in the Multiboot2 memory map */
	memory_maps_entry_size: usize,
	/* Pointer to the Multiboot2 memory map */
	memory_maps: *const multiboot::MmapEntry,

	/* Pointer to the end of the physical memory */
	memory_end: *const Void,
	/* Pointer to the beginning of physical allocatable memory */
	phys_alloc_begin: *const Void,
	/* Pointer to the end of physical allocatable memory */
	phys_alloc_end: *const Void,
	/* The amount total of allocatable memory */
	available_memory: usize,
}

/*
 * Variable containing the memory mapping.
 */
static mut MEM_INFO: MemoryInfo = MemoryInfo {
	memory_maps_size: 0,
	memory_maps_entry_size: 0,
	memory_maps: 0 as *const _,
	memory_end: 0 as *const _,
	phys_alloc_begin: 0 as *const _,
	phys_alloc_end: 0 as *const _,
	available_memory: 0,
};

/*
 * Returns the structure storing memory mapping informations.
 */
pub fn get_info() -> &'static MemoryInfo {
	unsafe {
		&MEM_INFO
	}
}

/*
 * Tells if a Multiboot mmap entry is valid.
 */
fn is_valid_entry(entry: &multiboot::MmapEntry) -> bool {
	entry.addr + entry.len < (1 as u64) << (4 * 8)
}

/*
 * Prints the memory mapping.
 */
pub fn print_entries() {
	let mem_info = get_info();
	assert!(mem_info.memory_maps as usize != 0);

	::println!("--- Memory mapping ---");
	::println!("<begin> <end> <type>");

	let mut t = mem_info.memory_maps;
	while (t as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size as usize) {
		unsafe {
			if is_valid_entry(&*t) {
				let begin = (*t).addr as *const Void;
				let end = (((*t).addr as usize) + ((*t).len as usize)) as *const Void;
				let type_ = get_type_string((*t).type_);
				::println!("- {:p} {:p} {}", begin, end, type_);
			}
		}
		t = ((t as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
}

/*
 * Returns a pointer to the beginning of the allocatable physical memory.
 */
fn get_phys_alloc_begin(multiboot_ptr: *const Void) -> *const Void {
	let multiboot_tags_size = multiboot::get_tags_size(multiboot_ptr);
	let multiboot_tags_end = ((multiboot_ptr as usize) + multiboot_tags_size) as *const _;
	let ptr = max(multiboot_tags_end, memory::get_kernel_end());
	// TODO ELF
	// ptr = util::max(ptr, boot_info.phys_elf_sections + boot_info.elf_num * sizeof(elf_section_header_t));
	return util::align(ptr, memory::PAGE_SIZE);
}

/*
 * Returns a pointer to the end of the system memory.
 */
fn get_memory_end() -> *const Void {
	let mem_info = get_info();
	assert!((mem_info.memory_maps as usize) != 0);

	let mut t = mem_info.memory_maps;
	let mut end: usize = 0;

	while (t as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size as usize) {
		unsafe {
			if is_valid_entry(&*t) {
				end = max(end, ((*t).addr as usize) + ((*t).len as usize));
			}
		}
		t = ((t as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
	return util::down_align(end as *const _, memory::PAGE_SIZE);
}

/*
 * Fills the memory mapping structure according to Multiboot's informations.
 */
pub fn init(multiboot_ptr: *const Void) {
	let boot_info = multiboot::get_boot_info();

	unsafe {
		MEM_INFO.memory_maps_size = boot_info.memory_maps_size;
		MEM_INFO.memory_maps_entry_size = boot_info.memory_maps_entry_size;
		MEM_INFO.memory_maps = boot_info.memory_maps;
		MEM_INFO.memory_end = get_memory_end();
		MEM_INFO.phys_alloc_begin = get_phys_alloc_begin(multiboot_ptr);
		MEM_INFO.phys_alloc_end = util::down_align((boot_info.mem_upper * 1024) as *const _,
			memory::PAGE_SIZE);
		if MEM_INFO.phys_alloc_begin >= MEM_INFO.phys_alloc_end {
			panic::kernel_panic("Invalid memory map!", 0);
		}
		MEM_INFO.available_memory = (MEM_INFO.phys_alloc_end as usize)
			- (MEM_INFO.phys_alloc_begin as usize);
	}
}

/*
 * Returns the string describing a memory region according to its type.
 */
fn get_type_string(t: u32) -> &'static str {
	match t {
		multiboot::MEMORY_AVAILABLE => "Available",
		multiboot::MEMORY_RESERVED => "Reserved",
		multiboot::MEMORY_ACPI_RECLAIMABLE => "ACPI",
		multiboot::MEMORY_NVS => "Hibernate",
		multiboot::MEMORY_BADRAM => "Bad RAM",
		_ => "Unknown",
	}
}
