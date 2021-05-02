/// This file handles memory allocators initialization for the kernel. Memory allocators are
/// located in the library `mem_alloc`.

use core::ffi::c_void;
use crate::memory::buddy;
use crate::memory::memmap;
use crate::memory;
use crate::util;

// TODO Clean
/// Initializes the memory allocators.
pub fn init() {
	buddy::prepare();
	let mmap_info = memmap::get_info();

	let virt_alloc_begin = memory::kern_to_virt(mmap_info.phys_alloc_begin);
	let metadata_begin = util::align(virt_alloc_begin, memory::PAGE_SIZE) as *mut c_void;
	let frames_count = mmap_info.available_memory
		/ (memory::PAGE_SIZE + buddy::get_frame_metadata_size());
	let metadata_size = frames_count * buddy::get_frame_metadata_size();
	let metadata_end = unsafe {
		metadata_begin.add(metadata_size)
	};
	let phys_metadata_end = memory::kern_to_phys(metadata_end);
	// TODO Check that metadata doesn't exceed kernel space's capacity

	let kernel_zone_begin = util::align(phys_metadata_end, memory::PAGE_SIZE) as *mut c_void;
	let kernel_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_KERNEL, metadata_begin,
		frames_count as _, kernel_zone_begin);
	let user_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_USER, 0 as *mut c_void, 0,
		0 as *mut c_void);
	let dma_zone = buddy::Zone::new(buddy::FLAG_ZONE_TYPE_DMA, 0 as *mut c_void, 0,
		0 as *mut c_void);
	buddy::set_zone_slot(1, kernel_zone);
	buddy::set_zone_slot(0, user_zone);
	buddy::set_zone_slot(2, dma_zone);
}
