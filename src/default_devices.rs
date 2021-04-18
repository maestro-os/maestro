/// This modules implements the default devices handles.

use crate::errno::Errno;
use crate::filesystem::device::DeviceHandle;

/// Structure representing a device which does nothing.
pub struct NullDeviceHandle {}

impl DeviceHandle for NullDeviceHandle {
	fn read(&mut self, _offset: usize, _buff: &mut [u8]) -> Result<usize, Errno> {
		Ok(0)
	}

	fn write(&mut self, _offset: usize, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len())
	}
}

/// Structure representing a device gives null bytes.
pub struct ZeroDeviceHandle {}

impl DeviceHandle for ZeroDeviceHandle {
	fn read(&mut self, _offset: usize, buff: &mut [u8]) -> Result<usize, Errno> {
		for i in 0..buff.len() {
			buff[i] = 0;
		}
		Ok(buff.len())
	}

	fn write(&mut self, _offset: usize, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len())
	}
}
