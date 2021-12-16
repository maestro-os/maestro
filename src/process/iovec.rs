//! This module implements the IOVec structure.

use core::ffi::c_void;

/// Structure used to represent an entry of an IO vector used for sparse buffers IO.
#[repr(C)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}
