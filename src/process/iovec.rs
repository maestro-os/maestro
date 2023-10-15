//! I/O vectors allow passing several buffers at once to a system call.
//!
//! This feature allows reducing the overhead linked to context switches.

use core::ffi::c_void;

/// An entry of an IO vector used for sparse buffers IO.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}

// TODO add a function to turn into an entry into a SyscallSlice?
