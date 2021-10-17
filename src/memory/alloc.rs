//! This file handles memory allocators initialization for the kernel.
//! The physical memory is divided into zones. Each zones contains frames that can be allocated by
//! the buddy allocator
//!
//! The following zones exist:
//! - Kernel: Memory to be allocated by the kernel, shared accross processes. This zone requires
//! that every frames of virtual memory are associated with a unique physical frame.
//! - MMIO: Memory used for Memory Mapped I/O. This zones requires only virtual memory, thus it
//! overlaps with the user zone which allocates the physical memory.
//! - User: Memory used for userspace mappings. This zone doesn't requires virtual memory to
//! correspond with the physical memory, thus it can be located outside of the kernelspace.

use core::cmp::min;
use core::ffi::c_void;
use crate::memory::buddy;
use crate::memory::memmap;
use crate::memory;
use crate::util;

/// Initializes the memory allocators.
pub fn init() {
	unsafe {
		buddy::prepare();
	}
	let mmap_info = memmap::get_info();

	// The pointer to the beginning of available memory
	let virt_alloc_begin = memory::kern_to_virt(mmap_info.phys_alloc_begin);
	// The pointer to the beginning of the buddy allocator's metadata
	let metadata_begin = util::align(virt_alloc_begin, memory::PAGE_SIZE) as *mut c_void;
	// The total number of allocatable frames
	let frames_count = mmap_info.available_memory
		/ (memory::PAGE_SIZE + buddy::get_frame_metadata_size());
	// The size of the buddy allocator's metadata
	let metadata_size = frames_count * buddy::get_frame_metadata_size();
	// The end of the buddy allocator's metadata
	let metadata_end = unsafe {
		metadata_begin.add(metadata_size)
	};
	// The physical address of the end of the buddy allocator's metadata
	let phys_metadata_end = memory::kern_to_phys(metadata_end);



	// The beginning of the kernel's zone
	let kernel_zone_begin = util::align(phys_metadata_end, memory::PAGE_SIZE) as *mut c_void;
	// The maximum number of pages the kernel zone can hold.
	let kernel_max = (memory::get_kernelspace_size()
		- (metadata_end as usize - memory::PROCESS_END as usize))
			/ memory::PAGE_SIZE;
	// The number of frames the kernel zone holds.
	let kernel_zone_frames = min(frames_count, kernel_max);
	// The kernel's zone
	let kernel_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_KERNEL, metadata_begin,
		kernel_zone_frames as _, kernel_zone_begin);
	buddy::set_zone_slot(buddy::FLAG_ZONE_TYPE_KERNEL as _, kernel_zone);



	// The beginning of the userspace's zone
	let userspace_zone_begin = unsafe {
		kernel_zone_begin.add(kernel_zone_frames * memory::PAGE_SIZE)
	};
	// The beginning of the userspace zone's metadata
	let userspace_metadata_begin = unsafe {
		metadata_begin.add(kernel_zone_frames * buddy::get_frame_metadata_size())
	};
	// The number of frames the userspace holds.
	let userspace_frames = frames_count - kernel_zone_frames;
	let user_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_USER, userspace_metadata_begin,
		userspace_frames as _, userspace_zone_begin);
	buddy::set_zone_slot(buddy::FLAG_ZONE_TYPE_USER as _, user_zone);



	// TODO MMIO zone
}
