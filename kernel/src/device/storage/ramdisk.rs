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
	device::{id, BlkDev, BlockDeviceOps, DeviceID, DeviceType},
	memory::RcFrame,
	sync::mutex::Mutex,
};
use core::{mem::ManuallyDrop, num::NonZeroU64};
use utils::{
	boxed::Box,
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::EResult,
	format,
	limits::PAGE_SIZE,
};

/// The ramdisks major number
const RAM_DISK_MAJOR: u32 = 1;
/// The number of ramdisks on the system
const RAM_DISK_COUNT: usize = 16;
/// The maximum size of the ramdisk in pages
const MAX_PAGES: u64 = 1024;

/// A disk, on RAM.
#[derive(Debug, Default)]
pub struct RamDisk(Mutex<Vec<RcFrame>>);

impl BlockDeviceOps for RamDisk {
	fn block_size(&self) -> NonZeroU64 {
		(PAGE_SIZE as u64).try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		MAX_PAGES
	}

	fn read_frame(&self, off: u64) -> EResult<RcFrame> {
		// Bound check
		if off >= MAX_PAGES {
			return Err(errno!(EOVERFLOW));
		}
		let off = off as usize;
		let mut pages = self.0.lock();
		// If the RAM is not large enough, expand it
		while pages.len() <= off {
			pages.push(RcFrame::new_zeroed()?)?;
		}
		Ok(pages[off].clone())
	}

	fn write_frame(&self, off: u64, _buf: &[u8]) -> EResult<()> {
		// Nothing to do, just check offset
		if off < MAX_PAGES {
			Ok(())
		} else {
			Err(errno!(EOVERFLOW))
		}
	}
}

/// Creates every ramdisk instances.
pub(crate) fn create() -> EResult<()> {
	let _major = ManuallyDrop::new(id::alloc_major(DeviceType::Block, Some(RAM_DISK_MAJOR))?);

	for i in 0..RAM_DISK_COUNT {
		let path = PathBuf::try_from(format!("/dev/ram{i}")?)?;
		let dev = BlkDev::new(
			DeviceID {
				major: RAM_DISK_MAJOR,
				minor: i as _,
			},
			path,
			0o666,
			Box::new(RamDisk::default())?,
		)?;
		device::register_blk(dev)?;
	}

	Ok(())
}
