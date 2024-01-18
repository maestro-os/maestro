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

//! This module handles the memory information, which stores global
//! information on the system memory by retrieving them from the boot
//! information. These data are meant to be used by the memory allocators.

use super::*;
use crate::elf::kernel::sections;
use crate::multiboot;
use crate::util;
use core::cmp::*;
use core::ffi::c_void;
use core::iter;
use core::mem::MaybeUninit;
use core::ptr::null;

/// Structure storing information relative to the main memory.
#[derive(Debug)]
pub struct MemoryInfo {
	/// Size of the Multiboot2 memory map
	pub memory_maps_size: usize,
	/// Size of an entry in the Multiboot2 memory map
	pub memory_maps_entry_size: usize,
	/// Pointer to the Multiboot2 memory map
	pub memory_maps: *const multiboot::MmapEntry,

	/// Pointer to the beginning of the main block of physical allocatable
	/// memory, page aligned.
	pub phys_main_begin: *const c_void,
	/// The size of the main block of physical allocatable memory, in pages.
	pub phys_main_pages: usize,
}

/// Variable containing the memory mapping.
static mut MEM_INFO: MaybeUninit<MemoryInfo> = MaybeUninit::uninit();

/// Returns the structure storing memory mapping information.
pub fn get_info() -> &'static MemoryInfo {
	unsafe { MEM_INFO.assume_init_mut() }
}

/// Prints the physical memory mapping.
#[cfg(config_debug_debug)]
pub(crate) fn print_entries() {
	let mem_info = get_info();
	debug_assert!(!mem_info.memory_maps.is_null());

	crate::println!("--- Memory mapping ---");
	crate::println!("<begin> <end> <type>");

	let mut ptr = mem_info.memory_maps;
	while (ptr as usize) < (mem_info.memory_maps as usize) + (mem_info.memory_maps_size) {
		let entry = unsafe { &*ptr };

		if entry.is_valid() {
			let begin = entry.addr;
			let end = begin + entry.len;
			let type_ = entry.get_type_string();
			crate::println!("- {begin:08x} {end:08x} {type_}");
		}

		ptr = ((ptr as usize) + mem_info.memory_maps_entry_size) as *const _;
	}
}

/// Computes and returns the physical address to the end of the kernel's ELF sections' content.
fn sections_end() -> *const c_void {
	let boot_info = multiboot::get_boot_info();
	// The end of ELF sections list
	let sections_list_end = (boot_info.elf_sections as usize
		+ boot_info.elf_num as usize * boot_info.elf_entsize as usize)
		as *const c_void;
	sections()
		// Get end of sections' content
		.map(|hdr| {
			let ptr = (hdr.sh_addr as usize + hdr.sh_size as usize) as *const c_void;
			kern_to_phys(ptr)
		})
		.chain(iter::once(sections_list_end))
		.max()
		.unwrap_or(null())
}

/// Returns the pointer to the beginning of the main physical allocatable memory
/// and its size in number of pages.
fn get_phys_main(multiboot_ptr: *const c_void) -> (*const c_void, usize) {
	let boot_info = multiboot::get_boot_info();

	// The end of the kernel code
	let mut begin = get_kernel_end();

	let multiboot_tags_size = unsafe { multiboot::get_tags_size(multiboot_ptr) };
	// The end of multiboot tags
	let multiboot_tags_end = ((multiboot_ptr as usize) + multiboot_tags_size) as *const _;
	begin = max(begin, multiboot_tags_end);

	// The end of the ELF sections
	begin = max(begin, sections_end());

	// The end of the loaded initramfs, if any
	if let Some(initramfs) = boot_info.initramfs {
		let initramfs_begin = kern_to_phys(initramfs.as_ptr() as _);
		let initramfs_end = ((initramfs_begin as usize) + initramfs.len()) as *const c_void;
		begin = max(begin, initramfs_end);
	}

	// Page-align
	begin = unsafe { util::align(begin, PAGE_SIZE) };

	// TODO Handle 64-bits systems
	let pages = min((1000 + boot_info.mem_upper) / 4, 1024 * 1024) as usize
		- ((begin as usize) / PAGE_SIZE);
	(begin, pages)
}

/// Fills the memory mapping structure according to Multiboot's information.
pub(crate) fn init(multiboot_ptr: *const c_void) {
	let boot_info = multiboot::get_boot_info();
	let mem_info = unsafe { MEM_INFO.assume_init_mut() };

	mem_info.memory_maps_size = boot_info.memory_maps_size;
	mem_info.memory_maps_entry_size = boot_info.memory_maps_entry_size;
	mem_info.memory_maps = boot_info.memory_maps;

	let (main_begin, main_pages) = get_phys_main(multiboot_ptr);
	mem_info.phys_main_begin = main_begin;
	mem_info.phys_main_pages = main_pages;

	// Set memory stats
	let mut mem_info = stats::MEM_INFO.lock();
	mem_info.mem_total = min(boot_info.mem_upper, 4194304) as _; // TODO Handle 64-bits systems
	mem_info.mem_free = main_pages * 4;
}
