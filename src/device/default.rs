/// This module implements default devices.

use crate::device::Device;
use crate::filesystem::path::Path;
use crate::errno::Errno;
use crate::device::DeviceHandle;
use crate::device;
use super::DeviceType;

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

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	let null_path = Path::from_string("/dev/null")?;
	let null_device = Device::new(1, 3, null_path, 0666, DeviceType::Char, NullDeviceHandle {})?;
	device::register_device(null_device)?;

	let zero_path = Path::from_string("/dev/zero")?;
	let zero_device = Device::new(1, 3, zero_path, 0666, DeviceType::Char, ZeroDeviceHandle {})?;
	device::register_device(zero_device)?;

	// TODO

	Ok(())
}
