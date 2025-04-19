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

//! Kernel Memory allocators initialization.
//!
//! The physical memory is divided into zones. Each zones contain frames that
//! can be allocated by the buddy allocator
//!
//! The following zones exist:
//! - Kernel: Memory to be allocated by the kernel, shared across processes. This zone requires
//!   that every frame of virtual memory are associated with a unique physical frame.
//! - MMIO: Memory used for Memory Mapped I/O. This zones requires only virtual memory, thus it
//!   overlaps with the user zone which allocates the physical memory.
//! - User: Memory used for userspace mappings. This zone doesn't require virtual memory to
//!   correspond with the physical memory, thus it can be located outside the kernelspace.

use crate::memory::{buddy, memmap::PHYS_MAP, KERNELSPACE_SIZE};
use core::cmp::min;
use utils::limits::PAGE_SIZE;

/// Initializes the memory allocators.
pub(crate) fn init() {
	// The number of available physical memory pages
	let mut available_pages = PHYS_MAP.phys_main_pages;

	// The pointer to the beginning of the buddy allocator's metadata
	let metadata_begin = PHYS_MAP.phys_main_begin.align_to(PAGE_SIZE);
	let metadata_begin_virt = metadata_begin.kernel_to_virtual().unwrap();
	// The size of the buddy allocator's metadata
	let metadata_size = available_pages * buddy::FRAME_METADATA_SIZE;
	// The end of the buddy allocator's metadata
	let metadata_end = metadata_begin + metadata_size;

	// Update the number of available pages
	available_pages -= metadata_size.div_ceil(PAGE_SIZE);

	// The beginning of the kernel's zone
	let kernel_zone_begin = metadata_end.align_to(PAGE_SIZE);
	// The maximum number of pages the kernel zone can hold.
	let kernel_max = (KERNELSPACE_SIZE - metadata_end.0) / PAGE_SIZE;
	// The number of frames the kernel zone holds.
	let kernel_zone_frames = min(available_pages, kernel_max);
	// The kernel's zone
	let kernel_zone = buddy::Zone::new(
		metadata_begin_virt,
		kernel_zone_begin,
		kernel_zone_frames as _,
	);

	// Update the number of available pages
	available_pages -= kernel_zone_frames;

	// The beginning of the userspace's zone
	let userspace_zone_begin = kernel_zone_begin + kernel_zone_frames * PAGE_SIZE;
	// The beginning of the userspace zone's metadata
	let userspace_metadata_begin =
		metadata_begin_virt + kernel_zone_frames * buddy::FRAME_METADATA_SIZE;
	let user_zone = buddy::Zone::new(
		userspace_metadata_begin,
		userspace_zone_begin,
		available_pages as _,
	);

	// TODO MMIO zone

	*buddy::ZONES.lock() = [
		user_zone,
		unsafe { core::mem::zeroed() }, // TODO MMIO
		kernel_zone,
	];
}
