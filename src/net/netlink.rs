//! `netlink` is an interface between the kernel and userspace.

use crate::errno::Errno;

/// Netlink message header.
#[repr(C)]
struct NLMsgHdr {
	/// Length of message including header
	nlmsg_len: u32,
	/// Type of message content
	nlmsg_type: u16,
	/// Additional flags
	nlmsg_flags: u16,
	/// Sequence number
	nlmsg_seq: u32,
	/// Sender port ID
	nlmsg_pid: u32,
}

/// TODO doc
#[derive(Debug)]
pub struct Handle {
	/// TODO doc
	pub family: i32,

	// TODO buffers
}

impl Handle {
	/// Creates a new handle.
	pub fn new() -> Result<Self, Errno> {
		// TODO
		todo!();
	}
}

impl Handle {
	/// Reads data from the I/O and writes it into `buff`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&mut self, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		// TODO
		todo!();
	}

	/// Reads data from `buff` and writes it into the I/O.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, _buff: &[u8]) -> Result<u64, Errno> {
		// TODO
		todo!();
	}

	/// Tells whether the specified events are available on the I/O interface.
	///
	/// `mask` is a mask containing the mask of operations to check for.
	///
	/// The function returns the mask with available events set.
	pub fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
