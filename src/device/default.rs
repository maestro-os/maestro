//! This module implements default devices.

use core::cmp::min;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use crate::crypto::rand::rand;
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
		let guard = mutex.lock();

		guard.get().get_size() as _
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		let mutex = logger::get();
		let guard = mutex.lock();

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

/// The random device allows to get random bytes. This device will block reading until enough
/// entropy is available.
pub struct RandomDeviceHandle {}

impl DeviceHandle for RandomDeviceHandle {
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for RandomDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		if rand(buff).is_some() {
			Ok(buff.len())
		} else {
			Ok(0)
		}
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<usize, Errno> {
		// TODO Feed entropy?
		todo!();
	}
}

/// This device works exactly like the random device, except it doesn't block. If not enough
/// entropy is available, the output might not have a sufficient quality.
pub struct URandomDeviceHandle {}

impl DeviceHandle for URandomDeviceHandle {
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for URandomDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<usize, Errno> {
		// TODO Feed entropy?
		todo!();
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
		tty::current().lock().get_mut().write(buff);
		Ok(buff.len())
	}
}

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);

	let null_path = Path::from_str(b"/dev/null", false)?;
	let mut null_device = Device::new(1, 3, null_path, 0o666, DeviceType::Char,
		NullDeviceHandle {})?;
	null_device.create_file()?; // TODO remove?
	device::register_device(null_device)?;

	let zero_path = Path::from_str(b"/dev/zero", false)?;
	let mut zero_device = Device::new(1, 5, zero_path, 0o666, DeviceType::Char,
		ZeroDeviceHandle {})?;
	zero_device.create_file()?; // TODO remove?
	device::register_device(zero_device)?;

	let random_path = Path::from_str(b"/dev/random", false)?;
	let mut random_device = Device::new(1, 8, random_path, 0o666, DeviceType::Char,
		RandomDeviceHandle {})?;
	random_device.create_file()?; // TODO remove?
	device::register_device(random_device)?;

	let urandom_path = Path::from_str(b"/dev/urandom", false)?;
	let mut urandom_device = Device::new(1, 8, urandom_path, 0o666, DeviceType::Char,
		URandomDeviceHandle {})?;
	urandom_device.create_file()?; // TODO remove?
	device::register_device(urandom_device)?;

	let kmsg_path = Path::from_str(b"/dev/kmsg", false)?;
	let mut kmsg_device = Device::new(1, 11, kmsg_path, 0o600, DeviceType::Char,
		KMsgDeviceHandle {})?;
	kmsg_device.create_file()?; // TODO remove?
	device::register_device(kmsg_device)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);

	let current_tty_path = Path::from_str(b"/dev/tty", false)?;
	let mut current_tty_device = Device::new(5, 0, current_tty_path, 0o666, DeviceType::Char,
		CurrentTTYDeviceHandle {})?;
	current_tty_device.create_file()?; // TODO remove?
	device::register_device(current_tty_device)?;

	Ok(())
}
