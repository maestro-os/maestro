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

//! This file handles memory allocators initialization for the kernel.
//!
//! The physical memory is divided into zones. Each zones contain frames that
//! can be allocated by the buddy allocator
//!
//! The following zones exist:
//! - Kernel: Memory to be allocated by the kernel, shared across processes. This zone requires
//! that every frame of virtual memory are associated with a unique physical
//! frame.
//! - MMIO: Memory used for Memory Mapped I/O. This zones requires only virtual memory, thus it
//! overlaps with the user zone which allocates the physical memory.
//! - User: Memory used for userspace mappings. This zone doesn't require virtual memory to
//! correspond with the physical memory, thus it can be located outside the kernelspace.

use crate::{
	memory,
	memory::{buddy, memmap},
};
use core::{cmp::min, ffi::c_void};

/// Initializes the memory allocators.
pub(crate) fn init() {
	let phys_map = memmap::get_info();
	// The pointer to the beginning of available memory
	let virt_alloc_begin = memory::kern_to_virt(phys_map.phys_main_begin);
	// The number of available physical memory pages
	let mut available_pages = phys_map.phys_main_pages;

	// The pointer to the beginning of the buddy allocator's metadata
	let metadata_begin =
		unsafe { utils::align(virt_alloc_begin, memory::PAGE_SIZE) as *mut c_void };
	// The size of the buddy allocator's metadata
	let metadata_size = available_pages * buddy::get_frame_metadata_size();
	// The end of the buddy allocator's metadata
	let metadata_end = (metadata_begin as usize + metadata_size) as *mut c_void;
	// The physical address of the end of the buddy allocator's metadata
	let phys_metadata_end = memory::kern_to_phys(metadata_end);

	// Updating the number of available pages
	available_pages -= metadata_size.div_ceil(memory::PAGE_SIZE);

	// The beginning of the kernel's zone
	let kernel_zone_begin =
		unsafe { utils::align(phys_metadata_end, memory::PAGE_SIZE) as *mut c_void };
	// The maximum number of pages the kernel zone can hold.
	let kernel_max =
		(memory::get_kernelspace_size() - phys_metadata_end as usize) / memory::PAGE_SIZE;
	// The number of frames the kernel zone holds.
	let kernel_zone_frames = min(available_pages, kernel_max);
	// The kernel's zone
	let kernel_zone = buddy::Zone::new(metadata_begin, kernel_zone_frames as _, kernel_zone_begin);

	// Updating the number of available pages
	available_pages -= kernel_zone_frames;

	// The beginning of the userspace's zone
	let userspace_zone_begin =
		(kernel_zone_begin as usize + kernel_zone_frames * memory::PAGE_SIZE) as *mut c_void;
	// The beginning of the userspace zone's metadata
	let userspace_metadata_begin = (metadata_begin as usize
		+ kernel_zone_frames * buddy::get_frame_metadata_size())
		as *mut c_void;
	let user_zone = buddy::Zone::new(
		userspace_metadata_begin,
		available_pages as _,
		userspace_zone_begin,
	);

	// TODO MMIO zone

	*buddy::ZONES.lock() = [
		user_zone,
		unsafe { core::mem::zeroed() }, // TODO MMIO
		kernel_zone,
	];
}
