//! This file handles memory allocators initialization for the kernel.

use core::cmp::min;
use core::ffi::c_void;
use core::ptr::null_mut;
use crate::memory::buddy;
use crate::memory::memmap;
use crate::memory;
use crate::util;

/// Initializes the memory allocators.
pub fn init() {
	buddy::prepare();
	let mmap_info = memmap::get_info();

	// The maximum number of pages the kernel zone can hold.
	let kernel_max = memory::get_kernelspace_size() / memory::PAGE_SIZE;

	// The pointer to the beginning of available memory
	let virt_alloc_begin = memory::kern_to_virt(mmap_info.phys_alloc_begin);
	// The pointer to the beginning of the buddy allocator's metadata
	let metadata_begin = util::align(virt_alloc_begin, memory::PAGE_SIZE) as *mut c_void;
	// The number of allocatable frames
	let frames_count = min(mmap_info.available_memory
		/ (memory::PAGE_SIZE + buddy::get_frame_metadata_size()), kernel_max);
	// The size of the buddy allocator's metadata
	let metadata_size = frames_count * buddy::get_frame_metadata_size();
	// The end of the buddy allocator's metadata
	let metadata_end = unsafe {
		metadata_begin.add(metadata_size)
	};
	// The physical address of the end of the buddy allocator's metadata
	let phys_metadata_end = memory::kern_to_phys(metadata_end);
	// TODO Check that metadata doesn't exceed kernel space's capacity

	// The beginning of the kernel's zone
	let kernel_zone_begin = util::align(phys_metadata_end, memory::PAGE_SIZE) as *mut c_void;
	// The kernel's zone
	let kernel_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_KERNEL, metadata_begin,
		frames_count as _, kernel_zone_begin);
	buddy::set_zone_slot(buddy::FLAG_ZONE_TYPE_KERNEL as _, kernel_zone);

	// The userspace's zone
	// TODO
	let user_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_USER, null_mut::<c_void>(), 0,
		null_mut::<c_void>());
	buddy::set_zone_slot(buddy::FLAG_ZONE_TYPE_USER as _, user_zone);

	// The dma zone
	// TODO
	let dma_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_DMA, null_mut::<c_void>(), 0,
		null_mut::<c_void>());
	buddy::set_zone_slot(buddy::FLAG_ZONE_TYPE_DMA as _, dma_zone);
}
