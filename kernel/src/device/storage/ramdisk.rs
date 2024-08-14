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
	file::path::PathBuf,
};
use core::{mem::ManuallyDrop, num::NonZeroU64};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	format,
};

/// The ramdisks' major number.
const RAM_DISK_MAJOR: u32 = 1;
/// The number of ramdisks on the system.
const RAM_DISK_COUNT: usize = 16;
/// The size of the ramdisk in bytes.
const RAM_DISK_SIZE: usize = 4 * 1024 * 1024;

// TODO Add a mechanism to free when cleared?

/// A disk, on RAM.
struct RAMDisk {
	/// The ramdisk's data.
	data: Vec<u8>,
}

impl RAMDisk {
	/// Creates a new ramdisk.
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
		}
	}

	/// If not allocated, allocates the disk.
	fn allocate(&mut self) -> AllocResult<()> {
		self.data.resize(RAM_DISK_SIZE, 0)
	}
}

impl DeviceIO for RAMDisk {
	fn block_size(&self) -> NonZeroU64 {
		512.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		(RAM_DISK_SIZE as u64) / self.block_size().get()
	}

	fn read(&self, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let block_size = self.block_size().get();
		let blocks_count = self.blocks_count();
		let size = buf.len() as u64 / block_size;
		if off > blocks_count || off + size > blocks_count {
			return Err(errno!(EINVAL));
		}

		let off = off * block_size;
		for i in 0..size {
			for j in 0..block_size {
				let buf_index = (i * block_size + j) as usize;
				let disk_index = (off + buf_index as u64) as usize;

				buf[buf_index] = self.data[disk_index];
			}
		}

		Ok((size * block_size) as _)
	}

	fn write(&self, off: u64, buf: &[u8]) -> EResult<usize> {
		let block_size = self.block_size().get();
		let blocks_count = self.blocks_count();
		let size = buf.len() as u64 / block_size;
		if off > blocks_count || off + size > blocks_count {
			return Err(errno!(EINVAL));
		}

		self.allocate()?;

		let off = off * block_size;
		for i in 0..size {
			for j in 0..block_size {
				let buf_index = (i * block_size + j) as usize;
				let disk_index = (off + buf_index as u64) as usize;

				self.data[disk_index] = buf[buf_index];
			}
		}

		Ok((size * block_size) as _)
	}
}

/// Structure representing a device for a ram disk.
struct RAMDiskHandle {
	/// The ramdisk.
	disk: RAMDisk,
}

impl RAMDiskHandle {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			disk: RAMDisk::new(),
		}
	}
}

impl DeviceIO for RAMDiskHandle {
	fn block_size(&self) -> NonZeroU64 {
		1.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		self.disk.data.len() as _
	}

	fn read(&self, off: u64, buff: &mut [u8]) -> EResult<usize> {
		self.disk.read(off, buff)
	}

	fn write(&self, off: u64, buff: &[u8]) -> EResult<usize> {
		self.disk.write(off, buff)
	}
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
			RAMDiskHandle::new(),
		)?;
		device::register(dev)?;
	}

	Ok(())
}

/*#[cfg(test)]
mod test {
	use super::*;
	use kernel::cmp::min;

	#[test_case]
	fn ramdisk0() {
		let mut ramdisk = RAMDiskHandle::new();
		let mut buff: [u8; 512] = [0; 512];
		ramdisk.read(0, &mut buff).unwrap();

		for i in 0..buff.len() {
			assert_eq!(buff[i], 0);
		}
	}

	#[test_case]
	fn ramdisk1() {
		let mut ramdisk = RAMDiskHandle::new();
		let mut buff: [u8; 512] = [0; 512];

		for i in (0..RAM_DISK_SIZE).step_by(buff.len()) {
			let size = min(buff.len(), RAM_DISK_SIZE - i);
			ramdisk.read(i as _, &mut buff[0..size]).unwrap();

			for j in 0..size {
				assert_eq!(buff[j], 0);
			}
		}
	}

	#[test_case]
	fn ramdisk2() {
		let mut ramdisk = RAMDiskHandle::new();
		let mut buff: [u8; 512] = [0; 512];
		for i in 0..buff.len() {
			buff[i] = 1;
		}

		for i in (0..RAM_DISK_SIZE).step_by(buff.len()) {
			let size = min(buff.len(), RAM_DISK_SIZE - i);
			ramdisk.write(i as _, &mut buff[0..size]).unwrap();
		}

		for i in (0..RAM_DISK_SIZE).step_by(buff.len()) {
			let size = min(buff.len(), RAM_DISK_SIZE - i);
			ramdisk.read(i as _, &mut buff[0..size]).unwrap();

			for j in 0..size {
				assert_eq!(buff[j], 1);
			}
		}
	}

	#[test_case]
	fn ramdisk3() {
		let mut ramdisk = RAMDiskHandle::new();
		let mut buff: [u8; 100] = [0; 100];
		for i in 0..buff.len() {
			buff[i] = 1;
		}

		ramdisk.write(0, &mut buff).unwrap();

		for i in (0..RAM_DISK_SIZE).step_by(buff.len()) {
			let size = min(buff.len(), RAM_DISK_SIZE - i);
			ramdisk.read(i as _, &mut buff[0..size]).unwrap();

			for j in 0..size {
				let val = {
					if i == 0 {
						1
					} else {
						0
					}
				};

				assert_eq!(buff[j], val);
			}
		}
	}

	#[test_case]
	fn ramdisk4() {
		let mut ramdisk = RAMDiskHandle::new();
		let mut buff: [u8; 512] = [0; 512];
		for i in 0..buff.len() {
			buff[i] = 1;
		}

		ramdisk.write(42, &mut buff).unwrap();

		for i in (0..RAM_DISK_SIZE).step_by(buff.len()) {
			let size = min(buff.len(), RAM_DISK_SIZE - i);
			ramdisk.read(i as _, &mut buff[0..size]).unwrap();

			for j in 0..size {
				let val = {
					let abs_index = i + j;
					if abs_index >= 42 && abs_index < 42 + buff.len() {
						1
					} else {
						0
					}
				};

				assert_eq!(buff[j], val);
			}
		}
	}
}*/
