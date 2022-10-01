//! The initramfs is a tmpfs which stores initialization files. It is loaded when the kernel boots.

use core::ffi::c_void;
use crate::errno::Errno;

/// Loads and mounts the initramsfs.
///
/// - `begin` is the pointer in physical memory to the beginning of the initramfs image.
/// - `usize` is the size in bytes of the initramfs image.
pub fn load(begin: *const c_void, size: usize) -> Result<(), Errno> {
	// TODO
	crate::println!("-> {:p} {}", begin, size);
	todo!();
}
