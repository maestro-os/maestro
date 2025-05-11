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

use super::{CharDev, DeviceType, id, register_char};
use crate::{
	crypto::{
		rand,
		rand::{GRND_RANDOM, getrandom},
	},
	device::{DeviceID, tty::TTYDeviceHandle},
	file::{File, fs::FileOps},
	logger::LOGGER,
	memory::user::UserSlice,
};
use core::mem::ManuallyDrop;
use utils::{collections::path::PathBuf, errno, errno::EResult};

/// Device which does nothing.
#[derive(Debug)]
pub struct NullDeviceHandle;

impl FileOps for NullDeviceHandle {
	fn read(&self, _file: &File, _: u64, _buf: UserSlice<u8>) -> EResult<usize> {
		Ok(0)
	}

	fn write(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device returning only null bytes.
#[derive(Debug)]
pub struct ZeroDeviceHandle;

impl FileOps for ZeroDeviceHandle {
	fn read(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let b: [u8; 128] = [0; 128];
		let mut i = 0;
		while i < buf.len() {
			i += buf.copy_to_user(i, &b)?;
		}
		Ok(buf.len())
	}

	fn write(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		Ok(buf.len())
	}
}

/// Device allows to get random bytes.
///
/// This device will block reading until enough entropy is available.
#[derive(Debug)]
pub struct RandomDeviceHandle;

impl FileOps for RandomDeviceHandle {
	fn read(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		getrandom(buf, GRND_RANDOM)
	}

	fn write(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			// TODO make blocking if the pool is full?
			pool.write(buf)
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
	fn read(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		getrandom(buf, 0)
	}

	fn write(&self, _file: &File, _: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let mut pool = rand::ENTROPY_POOL.lock();
		if let Some(pool) = &mut *pool {
			pool.write(buf)
		} else {
			Err(errno!(EINVAL))
		}
	}
}

/// Device allowing to read or write kernel logs.
#[derive(Debug)]
pub struct KMsgDeviceHandle;

impl FileOps for KMsgDeviceHandle {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let off = off.try_into().map_err(|_| errno!(EINVAL))?;
		let logger = LOGGER.lock();
		let content = logger.get_content();
		let l = buf.copy_to_user(0, &content[off..])?;
		Ok(l)
	}

	fn write(&self, _file: &File, _off: u64, _buf: UserSlice<u8>) -> EResult<usize> {
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
