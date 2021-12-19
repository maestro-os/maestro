//! This module implements default devices.

use core::cmp::min;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::logger;
use crate::tty;
use crate::util::IO;
use super::DeviceType;
use super::id;

/// Structure representing a device which does nothing.
pub struct NullDeviceHandle {}

impl DeviceHandle for NullDeviceHandle {
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for NullDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		Ok(0)
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		Ok(buff.len() as _)
	}
}

/// Structure representing a device gives null bytes.
pub struct ZeroDeviceHandle {}

impl DeviceHandle for ZeroDeviceHandle {
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for ZeroDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
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
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for KMsgDeviceHandle {
	fn get_size(&self) -> u64 {
		let mutex = logger::get();
		let guard = mutex.lock(true);

		guard.get().get_size() as _
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		let mutex = logger::get();
		let guard = mutex.lock(true);

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
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for CurrentTTYDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO Read from TTY input
		todo!();
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		tty::current().lock(true).get_mut().write(buff);
		Ok(buff.len())
	}
}

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);

	let null_path = Path::from_str("/dev/null".as_bytes(), false)?;
	let mut null_device = Device::new(1, 3, null_path, 0o666, DeviceType::Char,
		NullDeviceHandle {})?;
	null_device.create_file()?; // TODO remove?
	device::register_device(null_device)?;

	let zero_path = Path::from_str("/dev/zero".as_bytes(), false)?;
	let mut zero_device = Device::new(1, 5, zero_path, 0o666, DeviceType::Char,
		ZeroDeviceHandle {})?;
	zero_device.create_file()?; // TODO remove?
	device::register_device(zero_device)?;

	let kmsg_path = Path::from_str("/dev/kmsg".as_bytes(), false)?;
	let mut kmsg_device = Device::new(1, 11, kmsg_path, 0o600, DeviceType::Char,
		KMsgDeviceHandle {})?;
	kmsg_device.create_file()?; // TODO remove?
	device::register_device(kmsg_device)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);

	let current_tty_path = Path::from_str("/dev/tty".as_bytes(), false)?;
	let mut current_tty_device = Device::new(5, 0, current_tty_path, 0o666, DeviceType::Char,
		CurrentTTYDeviceHandle {})?;
	current_tty_device.create_file()?; // TODO remove?
	device::register_device(current_tty_device)?;

	Ok(())
}
