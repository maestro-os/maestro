//! This module implements the Input/Output interface trait.

use crate::errno::Errno;

/// Poll event: There is data to read.
pub const POLLIN: u32 = 0x1;
/// Poll event: There is some exceptional condition on the file descriptor.
pub const POLLPRI: u32 = 0x2;
/// Poll event: Writing is now possible.
pub const POLLOUT: u32 = 0x4;
/// Poll event: Error condition.
pub const POLLERR: u32 = 0x8;
/// Poll event: Hang up.
pub const POLLHUP: u32 = 0x10;
/// Poll event: Invalid request.
pub const POLLNVAL: u32 = 0x20;
/// Poll event: Equivalent to POLLIN.
pub const POLLRDNORM: u32 = 0x40;
/// Poll event: Priority band data can be read.
pub const POLLRDBAND: u32 = 0x80;
/// Poll event: Equivalent to POLLOUT.
pub const POLLWRNORM: u32 = 0x100;
/// Poll event: Priority data may be written.
pub const POLLWRBAND: u32 = 0x200;
/// Poll event: Stream socket peer closed connection, or shut down writing half
/// of connection.
pub const POLLRDHUP: u32 = 0x2000;

/// Trait representing a data I/O interface.
pub trait IO {
	/// Returns the size of the underlying data.
	fn get_size(&self) -> u64;

	/// Reads data from the I/O and writes it into `buff`.
	///
	/// `offset` is the offset in the I/O to the beginning of the data to be read.
	///
	/// The function returns a tuple containing:
	/// - The number of bytes read.
	/// - Whether the function reached the end of the input stream. In the context of a file, a
	/// value of `true` is equivalent to the End Of File (EOF).
	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno>;

	/// Reads data from `buff` and writes it into the I/O.
	///
	/// `offset` is the offset in the I/O to the beginning of the data to write.
	///
	/// The function returns the number of bytes written.
	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<u64, Errno>;

	/// Tells whether the specified events are available on the I/O interface.
	///
	/// `mask` is a mask containing the mask of operations to check for.
	///
	/// The function returns the mask with available events set.
	fn poll(&mut self, mask: u32) -> Result<u32, Errno>;
}

/// Structure representing a dummy I/O interface.
pub struct DummyIO {}

impl IO for DummyIO {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		Ok((0, true))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Ok(0)
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(0)
	}
}
