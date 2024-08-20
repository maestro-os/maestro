/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements default devices.

use super::{id, DeviceIO, DeviceType};
use crate::{
	crypto::rand,
	device,
	device::{tty::TTYDeviceHandle, Device, DeviceID},
	file::path::PathBuf,
	logger::LOGGER,
};
use core::{cmp::min, mem::ManuallyDrop, num::NonZeroU64};
use utils::{errno, errno::EResult};

/// Device which does nothing.
pub struct NullDeviceHandle;

impl DeviceIO for NullDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, _off: u64, buf: &mut [u8]) -> EResult<usize> {
		Ok(buf.len())
	}

	fn write(&self, _off: u64, buf: &[u8]) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device returning only null bytes.
pub struct ZeroDeviceHandle;

impl DeviceIO for ZeroDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, buf: &mut [u8]) -> EResult<usize> {
		buf.fill(0);
		Ok(buf.len())
	}

	fn write(&self, _offset: u64, buf: &[u8]) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device allows to get random bytes.
///
/// This device will block reading until enough entropy is available.
pub struct RandomDeviceHandle;

impl DeviceIO for RandomDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, _: u64, buf: &mut [u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			// TODO actual make this device blocking
			Ok(pool.read(buf, false))
		} else {
			Ok(0)
		}
	}

	fn write(&self, _: u64, buf: &[u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			// TODO actual make this device blocking
			let len = pool.write(buf);
			Ok(len as _)
		} else {
			Err(errno!(EINVAL))
		}
	}
}

/// This device works exactly like [`RandomDeviceHandle`], except it does not block.
///
/// If not enough entropy is available, the output might not have a sufficient
/// quality.
pub struct URandomDeviceHandle;

impl DeviceIO for URandomDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, _: u64, buf: &mut [u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			let len = pool.read(buf, true);
			Ok(len)
		} else {
			Ok(0)
		}
	}

	fn write(&self, _: u64, buf: &[u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			let len = pool.write(buf);
			Ok(len)
		} else {
			Err(errno!(EINVAL))
		}
	}
}

/// Device allowing to read or write kernel logs.
pub struct KMsgDeviceHandle;

impl DeviceIO for KMsgDeviceHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		0
	}

	fn read(&self, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let off = off.try_into().map_err(|_| errno!(EINVAL))?;
		let logger = LOGGER.lock();
		let size = logger.get_size();
		let content = logger.get_content();

		let len = min(size - off, buf.len());
		buf[..len].copy_from_slice(&content[off..(off + len)]);
		Ok(len)
	}

	fn write(&self, _off: u64, _buf: &[u8]) -> EResult<usize> {
		// TODO
		todo!();
	}
}

/// Creates the default devices.
pub(super) fn create() -> EResult<()> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);

	let null_path = PathBuf::try_from(b"/dev/null")?;
	let null_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 1,
			minor: 3,
		},
		null_path,
		0o666,
		NullDeviceHandle,
	)?;
	device::register(null_device)?;

	let zero_path = PathBuf::try_from(b"/dev/zero")?;
	let zero_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 1,
			minor: 5,
		},
		zero_path,
		0o666,
		ZeroDeviceHandle,
	)?;
	device::register(zero_device)?;

	let random_path = PathBuf::try_from(b"/dev/random")?;
	let random_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 1,
			minor: 8,
		},
		random_path,
		0o666,
		RandomDeviceHandle,
	)?;
	device::register(random_device)?;

	let urandom_path = PathBuf::try_from(b"/dev/urandom")?;
	let urandom_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 1,
			minor: 9,
		},
		urandom_path,
		0o666,
		URandomDeviceHandle,
	)?;
	device::register(urandom_device)?;

	let kmsg_path = PathBuf::try_from(b"/dev/kmsg")?;
	let kmsg_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 1,
			minor: 11,
		},
		kmsg_path,
		0o600,
		KMsgDeviceHandle,
	)?;
	device::register(kmsg_device)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);

	let current_tty_path = PathBuf::try_from(b"/dev/tty")?;
	let current_tty_device = Device::new(
		DeviceID {
			dev_type: DeviceType::Char,
			major: 5,
			minor: 0,
		},
		current_tty_path,
		0o666,
		TTYDeviceHandle,
	)?;
	device::register(current_tty_device)?;

	Ok(())
}
