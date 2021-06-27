//! This module implements default devices.

use core::cmp::min;
use core::mem::ManuallyDrop;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::logger;
use crate::tty;
use crate::util::lock::mutex::MutexGuard;
use crate::util::lock::mutex::TMutex;
use super::DeviceType;
use super::id;

/// Structure representing a device which does nothing.
pub struct NullDeviceHandle {}

impl DeviceHandle for NullDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

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
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		for b in buff.iter_mut() {
			*b = 0;
		}

		Ok(buff.len())
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len())
	}
}

/// Structure representing the kernel logs.
pub struct KMsgDeviceHandle {}

impl DeviceHandle for KMsgDeviceHandle {
	fn get_size(&self) -> u64 {
		let mutex = logger::get();
		let guard = MutexGuard::new(mutex);

		guard.get().get_size() as _
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		let mutex = logger::get();
		let guard = MutexGuard::new(mutex);

		let size = guard.get().get_size();
		let content = guard.get().get_content();

		let len = min(size, buff.len()) - offset as usize;
		buff.copy_from_slice(&content[(offset as usize)..(offset as usize + len)]);
		Ok(len)
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		// TODO Write to logger
		Ok(buff.len())
	}
}

/// Structure representing the current TTY.
pub struct CurrentTTYDeviceHandle {}

impl DeviceHandle for CurrentTTYDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO Read from TTY input
		todo!();
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		tty::current().lock().get_mut().write(buff);
		Ok(buff.len())
	}
}

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);

	let null_path = Path::from_string("/dev/null")?;
	device::register_device(Device::new(1, 3, null_path, 0o666, DeviceType::Char,
		NullDeviceHandle {})?)?;

	let zero_path = Path::from_string("/dev/zero")?;
	device::register_device(Device::new(1, 5, zero_path, 0o666, DeviceType::Char,
		ZeroDeviceHandle {})?)?;

	let kmsg_path = Path::from_string("/dev/kmsg")?;
	device::register_device(Device::new(1, 11, kmsg_path, 0o600, DeviceType::Char,
		KMsgDeviceHandle {})?)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);

	let current_tty_path = Path::from_string("/dev/tty")?;
	let mut current_tty_device = Device::new(5, 0, current_tty_path, 0o666, DeviceType::Char,
		CurrentTTYDeviceHandle {})?;
	current_tty_device.create_file()?; // TODO remove?
	device::register_device(current_tty_device)?;

	Ok(())
}
