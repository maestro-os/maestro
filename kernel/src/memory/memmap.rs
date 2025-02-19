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
//! information.
//!
//! This data is meant to be used by the memory allocators.

use super::{stats, PhysAddr, VirtAddr};
use crate::{elf::kernel::sections, multiboot, multiboot::BootInfo, sync::once::OnceInit};
use core::{cmp::min, iter};
use utils::limits::PAGE_SIZE;

/// Physical memory map information.
#[derive(Debug)]
pub struct PhysMapInfo {
	/// Size of the Multiboot2 memory map
	pub memory_maps_size: usize,
	/// Size of an entry in the Multiboot2 memory map
	pub memory_maps_entry_size: usize,
	/// Pointer to the Multiboot2 memory map
	pub memory_maps: *const multiboot::MmapEntry,

	/// The size of the physical memory in number of pages.
	pub memory_size: usize,

	/// Physical address to the beginning of the main block of allocatable memory, page aligned.
	pub phys_main_begin: PhysAddr,
	/// The size of the main block of physical allocatable memory, in pages.
	pub phys_main_pages: usize,
}

/// Physical memory map information.
pub static PHYS_MAP: OnceInit<PhysMapInfo> = unsafe { OnceInit::new() };

/// Prints the physical memory mapping.
#[cfg(debug_assertions)]
pub(crate) fn print_entries() {
	debug_assert!(!PHYS_MAP.memory_maps.is_null());
	crate::println!("--- Memory mapping ---");
	crate::println!("<begin> <end> <type>");
	for off in (0..PHYS_MAP.memory_maps_size).step_by(PHYS_MAP.memory_maps_entry_size) {
		// Safe because in range
		let entry = unsafe { &*PHYS_MAP.memory_maps.byte_add(off) };
		if entry.is_valid() {
			let begin = entry.addr;
			let end = begin + entry.len;
			let type_ = entry.get_type_string();
			crate::println!("- {begin:08x} {end:08x} {type_}");
		}
	}
}

/// Computes and returns the physical address to the end of the kernel's ELF sections' content.
fn sections_end(boot_info: &BootInfo) -> PhysAddr {
	// The end of ELF sections list
	let sections_list_end =
		boot_info.elf_sections + boot_info.elf_num as usize * boot_info.elf_entsize as usize;
	sections()
		// Get end of sections' content
		.filter_map(|hdr| {
			let addr = hdr.sh_addr as usize + hdr.sh_size as usize;
			VirtAddr(addr).kernel_to_physical()
		})
		.chain(iter::once(sections_list_end))
		.max()
		.unwrap_or_default()
}

/// Fills the memory mapping structure according to Multiboot's information.
pub(crate) fn init(boot_info: &BootInfo) {
	// The end address of the loaded initramfs
	let initramfs_end = boot_info
		.initramfs
		.map(|initramfs| {
			let initramfs_begin = VirtAddr::from(initramfs.as_ptr())
				.kernel_to_physical()
				.unwrap();
			initramfs_begin + initramfs.len()
		})
		.unwrap_or_default();
	// Compute the physical address of the beginning of allocatable memory
	let phys_main_begin = [boot_info.tags_end, sections_end(boot_info), initramfs_end]
		.into_iter()
		.max()
		.unwrap()
		.align_to(PAGE_SIZE);
	// The size of the physical memory in pages
	let memory_size = min(
		(1000 + boot_info.mem_upper as usize) / 4,
		usize::MAX / PAGE_SIZE,
	);
	// The number of physical page available for memory allocation
	let phys_main_pages = memory_size - phys_main_begin.0 / PAGE_SIZE;
	// Set memory information
	let phys_map = PhysMapInfo {
		memory_maps_size: boot_info.memory_maps_size,
		memory_maps_entry_size: boot_info.memory_maps_entry_size,
		memory_maps: boot_info.memory_maps,

		memory_size,

		phys_main_begin,
		phys_main_pages,
	};
	unsafe {
		OnceInit::init(&PHYS_MAP, phys_map);
	}
	// Update memory stats
	let mut stats = stats::MEM_INFO.lock();
	stats.mem_total = phys_main_pages * 4;
	stats.mem_free = phys_main_pages * 4;
}
