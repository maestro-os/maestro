/// TODO doc

use core::ffi::c_void;
use crate::filesystem::path::Path;
use crate::filesystem;
use crate::util;

/// TODO doc
pub const O_APPEND: u32 =    0b00000000000001;
/// TODO doc
pub const O_ASYNC: u32 =     0b00000000000010;
/// TODO doc
pub const O_CLOEXEC: u32 =   0b00000000000100;
/// TODO doc
pub const O_CREAT: u32 =     0b00000000001000;
/// TODO doc
pub const O_DIRECT: u32 =    0b00000000010000;
/// TODO doc
pub const O_DIRECTORY: u32 = 0b00000000100000;
/// TODO doc
pub const O_EXCL: u32 =      0b00000001000000;
/// TODO doc
pub const O_LARGEFILE: u32 = 0b00000010000000;
/// TODO doc
pub const O_NOATIME: u32 =   0b00000100000000;
/// TODO doc
pub const O_NOCTTY: u32 =    0b00001000000000;
/// TODO doc
pub const O_NOFOLLOW: u32 =  0b00010000000000;
/// TODO doc
pub const O_NONBLOCK: u32 =  0b00100000000000;
/// TODO doc
pub const O_SYNC: u32 =      0b01000000000000;
/// TODO doc
pub const O_TRUNC: u32 =     0b10000000000000;

/// The implementation of the `open` syscall.
pub fn open(regs: &util::Regs) -> u32 {
	let pathname = regs.ebx as *const c_void;
	let _flags = regs.ecx;
	let _mode = regs.edx as u16;

	let path = Path::from_string(unsafe { // Call to unsafe function
		util::ptr_to_str(pathname)
	});
	// TODO Concat path with process's path to get absolute path
	let _file = filesystem::get_file_from_path(&path);
	// TODO
	0
}
