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

use super::{id, register_char, CharDev, DeviceType};
use crate::{
	crypto::rand,
	device::{tty::TTYDeviceHandle, DeviceID},
	file::{fs::FileOps, File},
	logger::LOGGER,
};
use core::{cmp::min, mem::ManuallyDrop};
use utils::{collections::path::PathBuf, errno, errno::EResult};

/// Device which does nothing.
#[derive(Debug)]
pub struct NullDeviceHandle;

impl FileOps for NullDeviceHandle {
	fn read(&self, _file: &File, _off: u64, _buf: &mut [u8]) -> EResult<usize> {
		Ok(0)
	}

	fn write(&self, _file: &File, _off: u64, buf: &[u8]) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device returning only null bytes.
#[derive(Debug)]
pub struct ZeroDeviceHandle;

impl FileOps for ZeroDeviceHandle {
	fn read(&self, _file: &File, _offset: u64, buf: &mut [u8]) -> EResult<usize> {
		buf.fill(0);
		Ok(buf.len())
	}

	fn write(&self, _file: &File, _offset: u64, buf: &[u8]) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device allows to get random bytes.
///
/// This device will block reading until enough entropy is available.
#[derive(Debug)]
pub struct RandomDeviceHandle;

impl FileOps for RandomDeviceHandle {
	fn read(&self, _file: &File, _: u64, buf: &mut [u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			// TODO actual make this device blocking
			Ok(pool.read(buf, false))
		} else {
			Ok(0)
		}
	}

	fn write(&self, _file: &File, _: u64, buf: &[u8]) -> EResult<usize> {
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
#[derive(Debug)]
pub struct URandomDeviceHandle;

impl FileOps for URandomDeviceHandle {
	fn read(&self, _file: &File, _: u64, buf: &mut [u8]) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			let len = pool.read(buf, true);
			Ok(len)
		} else {
			Ok(0)
		}
	}

	fn write(&self, _file: &File, _: u64, buf: &[u8]) -> EResult<usize> {
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
#[derive(Debug)]
pub struct KMsgDeviceHandle;

impl FileOps for KMsgDeviceHandle {
	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let off = off.try_into().map_err(|_| errno!(EINVAL))?;
		let logger = LOGGER.lock();
		let size = logger.get_size();
		let content = logger.get_content();

		let len = min(size - off, buf.len());
		buf[..len].copy_from_slice(&content[off..(off + len)]);
		Ok(len)
	}

	fn write(&self, _file: &File, _off: u64, _buf: &[u8]) -> EResult<usize> {
		todo!();
	}
}

/// Creates the default devices.
pub(super) fn create() -> EResult<()> {
	let _first_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(1))?);
	register_char(CharDev::new(
		DeviceID {
			major: 1,
			minor: 3,
		},
		PathBuf::try_from(b"/dev/null")?,
		0o666,
		NullDeviceHandle,
	)?)?;
	register_char(CharDev::new(
		DeviceID {
			major: 1,
			minor: 5,
		},
		PathBuf::try_from(b"/dev/zero")?,
		0o666,
		ZeroDeviceHandle,
	)?)?;
	register_char(CharDev::new(
		DeviceID {
			major: 1,
			minor: 8,
		},
		PathBuf::try_from(b"/dev/random")?,
		0o666,
		RandomDeviceHandle,
	)?)?;
	register_char(CharDev::new(
		DeviceID {
			major: 1,
			minor: 9,
		},
		PathBuf::try_from(b"/dev/urandom")?,
		0o666,
		URandomDeviceHandle,
	)?)?;
	register_char(CharDev::new(
		DeviceID {
			major: 1,
			minor: 11,
		},
		PathBuf::try_from(b"/dev/kmsg")?,
		0o600,
		KMsgDeviceHandle,
	)?)?;

	let _fifth_major = ManuallyDrop::new(id::alloc_major(DeviceType::Char, Some(5))?);
	register_char(CharDev::new(
		DeviceID {
			major: 5,
			minor: 0,
		},
		PathBuf::try_from(b"/dev/tty")?,
		0o666,
		TTYDeviceHandle,
	)?)?;

	Ok(())
}
