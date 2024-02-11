//! `netlink` is an interface between the kernel and userspace.

use crate::{
	errno::Errno,
	util::collections::{ring_buffer::RingBuffer, vec::Vec},
};
use core::mem::size_of;

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

/// The netlink handle for a socket. Each socket must have its own instance.
#[derive(Debug)]
pub struct Handle {
	/// Socket family being used
	pub family: i32,

	/// The buffer for read operations.
	read_buff: RingBuffer<u8, Vec<u8>>,
	/// The buffer for write operations.
	write_buff: RingBuffer<u8, Vec<u8>>,
}

impl Handle {
	/// Creates a new handle.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			family: 0,

			read_buff: RingBuffer::new(crate::vec![0; 16384]?),
			write_buff: RingBuffer::new(crate::vec![0; 16384]?),
		})
	}
}

impl Handle {
	/// Reads data from the I/O and writes it into `buff`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&mut self, buff: &mut [u8]) -> Result<(usize, bool), Errno> {
		let len = self.read_buff.read(buff);
		Ok((len, false))
	}

	/// Reads data from `buff` and writes it into the I/O.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buff: &[u8]) -> Result<usize, Errno> {
		let len = self.write_buff.write(buff);

		// Read message header
		let mut buf: [u8; size_of::<NLMsgHdr>()] = [0; size_of::<NLMsgHdr>()];
		let l = self.write_buff.peek(buf.as_mut_slice());
		if l < buf.len() {
			return Ok(len);
		}
		let _hdr = buf.as_ptr() as *const NLMsgHdr;

		// TODO handle message
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
