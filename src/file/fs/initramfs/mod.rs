//! The initramfs is a tmpfs which stores initialization files. It is loaded when the kernel boots.

mod cpio;

use crate::errno::Errno;

/// Loads and mounts the initramsfs.
///
/// `data` is the slice of data representing the initramfs image.
pub fn load(_data: &[u8]) -> Result<(), Errno> {
	// TODO
	todo!();
}
