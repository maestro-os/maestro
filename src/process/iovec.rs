//! This module implements the IOVec structure.

use core::ffi::c_void;

/// An entry of an IO vector used for sparse buffers IO.
#[repr(C)]
#[derive(Debug)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}
