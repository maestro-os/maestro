//! This module implements default devices.

use core::cmp::min;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use crate::crypto::rand;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::tty::TTYDeviceHandle;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::logger;
use crate::process::mem_space::MemSpace;
use crate::util::io::IO;
use crate::util::io;
use crate::util::ptr::IntSharedPtr;
use super::DeviceType;
use super::id;

/// Structure representing a device which does nothing.
pub struct NullDeviceHandle {}

impl DeviceHandle for NullDeviceHandle {
	fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		Err(errno!(EINVAL))
	}
}

impl IO for NullDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		Ok((0, true))
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		Ok(buff.len() as _)
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(io::POLLIN | io::POLLOUT)
	}
}

/// Structure representing a device gives null bytes.
pub struct ZeroDeviceHandle {}

impl DeviceHandle for ZeroDeviceHandle {
	fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		Err(errno!(EINVAL))
	}
}

impl IO for ZeroDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		for b in buff.iter_mut() {
			*b = 0;
		}

		Ok((buff.len() as _, false))
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		Ok(buff.len() as _)
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(io::POLLIN | io::POLLOUT)
	}
}

/// Structure representing the kernel logs.
pub struct KMsgDeviceHandle {}

impl DeviceHandle for KMsgDeviceHandle {
	fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		Err(errno!(EINVAL))
	}
}

impl IO for KMsgDeviceHandle {
	fn get_size(&self) -> u64 {
		let mutex = logger::get();
		let guard = mutex.lock();

		guard.get().get_size() as _
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if offset > (usize::MAX as u64) {
			return Err(errno!(EINVAL));
		}

		let mutex = logger::get();
		let guard = mutex.lock();
		let logger = guard.get();

		let size = logger.get_size();
		let content = logger.get_content();

		let len = min(size - offset as usize, buff.len());
		buff[..len].copy_from_slice(&content[(offset as usize)..(offset as usize + len)]);

		let eof = offset as usize + len >= size;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		// TODO Write to logger
		Ok(buff.len() as _)
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}

/// The random device allows to get random bytes. This device will block reading until enough
/// entropy is available.
pub struct RandomDeviceHandle {}

impl DeviceHandle for RandomDeviceHandle {
	fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		Err(errno!(EINVAL))
	}
}

impl IO for RandomDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if let Some(source_mutex) = rand::get_source("random") {
			let source_guard = source_mutex.lock();
			let source = source_guard.get_mut();

			let mut i = 0;
			while i < buff.len() {
				i += source.consume_entropy(&mut buff[i..]);
			}

			Ok((buff.len() as _, false))
		} else {
			Ok((0, true))
		}
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		// TODO Feed entropy?
		todo!();
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(io::POLLIN | io::POLLOUT)
	}
}

/// This device works exactly like the random device, except it doesn't block. If not enough
/// entropy is available, the output might not have a sufficient quality.
pub struct URandomDeviceHandle {}

impl DeviceHandle for URandomDeviceHandle {
	fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		Err(errno!(EINVAL))
	}
}

impl IO for URandomDeviceHandle {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if let Some(source_mutex) = rand::get_source("urandom") {
			let source_guard = source_mutex.lock();
			let source = source_guard.get_mut();

			let mut i = 0;
			while i < buff.len() {
				i += source.consume_entropy(&mut buff[i..]);
			}

			Ok((buff.len() as _, false))
		} else {
			Ok((0, true))
		}
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		// TODO Feed entropy?
		todo!();
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(io::POLLIN | io::POLLOUT)
	}
}

/// Creates the default devices.
pub fn create() -> Result<(), Errno> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);

	let null_path = Path::from_str(b"/dev/null", false)?;
	let null_device = Device::new(
		1,
		3,
		null_path,
		0o666,
		DeviceType::Char,
		NullDeviceHandle {},
	)?;
	device::register_device(null_device)?;

	let zero_path = Path::from_str(b"/dev/zero", false)?;
	let zero_device = Device::new(
		1,
		5,
		zero_path,
		0o666,
		DeviceType::Char,
		ZeroDeviceHandle {},
	)?;
	device::register_device(zero_device)?;

	// TODO
	/*let random_path = Path::from_str(b"/dev/random", false)?;
	let random_device = Device::new(
		1,
		8,
		random_path,
		0o666,
		DeviceType::Char,
		RandomDeviceHandle {},
	)?;
	device::register_device(random_device)?;

	let urandom_path = Path::from_str(b"/dev/urandom", false)?;
	let urandom_device = Device::new(
		1,
		9,
		urandom_path,
		0o666,
		DeviceType::Char,
		URandomDeviceHandle {},
	)?;
	device::register_device(urandom_device)?;*/

	let kmsg_path = Path::from_str(b"/dev/kmsg", false)?;
	let kmsg_device = Device::new(
		1,
		11,
		kmsg_path,
		0o600,
		DeviceType::Char,
		KMsgDeviceHandle {},
	)?;
	device::register_device(kmsg_device)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);

	let current_tty_path = Path::from_str(b"/dev/tty", false)?;
	let current_tty_device = Device::new(
		5,
		0,
		current_tty_path,
		0o666,
		DeviceType::Char,
		TTYDeviceHandle::new(None),
	)?;
	device::register_device(current_tty_device)?;

	Ok(())
}
