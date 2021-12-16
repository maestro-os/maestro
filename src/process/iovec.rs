//! TODO doc

use core::ffi::c_void;

/// TODO doc
#[repr(C)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}
