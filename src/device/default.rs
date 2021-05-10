/// This module implements default devices.

use core::str;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device;
use crate::errno::Errno;
use crate::file::path::Path;
use super::DeviceType;

/// Structure representing a device which does nothing.
pub struct NullDeviceHandle {}

impl DeviceHandle for NullDeviceHandle {
	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		Ok(0)
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len())
	}
}

/// Structure representing a device gives null bytes.
pub struct ZeroDeviceHandle {}

impl DeviceHandle for ZeroDeviceHandle {
	fn read(&mut self, _offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		for i in 0..buff.len() {
			buff[i] = 0;
		}
		Ok(buff.len())
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len())
	}
}

/// Structure representing the current TTY.
pub struct CurrentTTYDeviceHandle {}

impl DeviceHandle for CurrentTTYDeviceHandle {
	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO Read from TTY input
		Ok(0)
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		// Invalid UTF8 isn't important since the TTY is supposed to write exactly the data it gets
		let s = unsafe {
			str::from_utf8_unchecked(buff)
		};

		crate::print!("{}", s);
		Ok(buff.len())
	}
}

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	// TODO Allocate major blocks

	let null_path = Path::from_string("/dev/null")?;
	device::register_device(Device::new(1, 3, null_path, 0666, DeviceType::Char,
		NullDeviceHandle {})?)?;

	let zero_path = Path::from_string("/dev/zero")?;
	device::register_device(Device::new(1, 3, zero_path, 0666, DeviceType::Char,
		ZeroDeviceHandle {})?)?;

	let current_tty_path = Path::from_string("/dev/tty")?;
	let mut current_tty_device = Device::new(5, 0, current_tty_path, 0666, DeviceType::Char,
		CurrentTTYDeviceHandle {})?;
	current_tty_device.create_file()?; // TODO remove?
	device::register_device(current_tty_device)?;

	// TODO

	Ok(())
}
