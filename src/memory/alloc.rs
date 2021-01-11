/// This file handles memory allocators initialization for the kernel. Memory allocators are
/// located in the library `mem_alloc`.

use core::ffi::c_void;
use core::mem::size_of;
use crate::memory::memmap;
use mem_alloc::buddy::*;
use mem_alloc::r#const::*;

/// Initializes the buddy allocator.
pub fn init() {
	unsafe {
		util::zero_object(&mut ZONES);
	}

	let mmap_info = memmap::get_info();
	let z = unsafe { // Assuming that a global variable is initialized
		ZONES.assume_init_mut()
	};

	let virt_alloc_begin = mem_alloc::kern_to_virt(mmap_info.phys_alloc_begin);
	let metadata_begin = util::align(virt_alloc_begin, PAGE_SIZE) as *mut c_void;
	let frames_count = mmap_info.available_memory / (PAGE_SIZE + size_of::<Frame>());
	let metadata_size = frames_count * size_of::<Frame>();
	let metadata_end = unsafe { // Pointer arithmetic
		metadata_begin.add(metadata_size)
	};
	let phys_metadata_end = mem_alloc::kern_to_phys(metadata_end);
	// TODO Check that metadata doesn't exceed kernel space's capacity

	let kernel_zone_begin = util::align(phys_metadata_end, PAGE_SIZE) as *mut c_void;
	z[1].lock().get_mut().init(FLAG_ZONE_TYPE_KERNEL, metadata_begin, frames_count as _,
		kernel_zone_begin);
	z[1].unlock();

	// TODO
	z[0].lock().get_mut().init(FLAG_ZONE_TYPE_USER, 0 as *mut c_void, 0, 0 as *mut c_void);
	z[0].unlock();

	// TODO
	z[2].lock().get_mut().init(FLAG_ZONE_TYPE_DMA, 0 as *mut c_void, 0, 0 as *mut c_void);
	z[2].unlock();
}
