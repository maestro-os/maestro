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

//! A ramdisk is a virtual storage device stored on the RAM. From the point of
//! view of the userspace, it works exactly the same.
//!
//! Ramdisks are lazily allocated, so they do not use much memory as long as they
//! are not used.

use crate::{
	device,
	device::{id, Device, DeviceID, DeviceIO, DeviceType},
};
use core::{cmp::max, mem::ManuallyDrop, num::NonZeroU64};
use utils::{
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::EResult,
	format,
	lock::Mutex,
};

/// The ramdisks' major number.
const RAM_DISK_MAJOR: u32 = 1;
/// The number of ramdisks on the system.
const RAM_DISK_COUNT: usize = 16;
/// The maximum size of the ramdisk in bytes.
const RAM_DISK_SIZE: usize = 4 * 1024 * 1024;

// TODO Add a mechanism to free when cleared?
// TODO allow concurrent I/O?

/// A disk, on RAM.
#[derive(Default)]
pub struct RAMDisk(Mutex<Vec<u8>>);

impl DeviceIO for RAMDisk {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		RAM_DISK_SIZE as u64
	}

	fn read(&self, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let end = off.saturating_add(buf.len() as u64);
		// Bound check
		if end > RAM_DISK_SIZE as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		let end = end as usize;
		// Copy
		let data = self.0.lock();
		buf.copy_from_slice(&data[off..end]);
		Ok(buf.len())
	}

	fn write(&self, off: u64, buf: &[u8]) -> EResult<usize> {
		let end = off.saturating_add(buf.len() as u64);
		// Bound check
		if end > RAM_DISK_SIZE as u64 {
			return Err(errno!(EINVAL));
		}
		let off = off as usize;
		let end = end as usize;
		let mut data = self.0.lock();
		// Adapt size
		let new_size = max(end, data.len());
		self.0.lock().resize(new_size, 0)?;
		// Copy
		data[off..end].copy_from_slice(buf);
		Ok(buf.len())
	}
}

/// Structure representing a device for a ram disk.
struct RAMDiskHandle {
	/// The ramdisk.
	disk: RAMDisk,
}

/// Creates every ramdisk instances.
pub(crate) fn create() -> EResult<()> {
	// TODO Undo all on fail?
	let _major = ManuallyDrop::new(id::alloc_major(DeviceType::Block, Some(RAM_DISK_MAJOR))?);

	for i in 0..RAM_DISK_COUNT {
		let path = PathBuf::try_from(format!("/dev/ram{i}")?)?;
		let dev = Device::new(
			DeviceID {
				dev_type: DeviceType::Block,
				major: RAM_DISK_MAJOR,
				minor: i as _,
			},
			path,
			0o666,
			RAMDisk::default(),
		)?;
		device::register(dev)?;
	}

	Ok(())
}
