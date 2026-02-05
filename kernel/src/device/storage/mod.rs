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

//! Storage management implementation.

mod ide;
mod nvme;
pub mod partition;
mod pata;

use crate::{
	device::{
		BlkDev, BlockDeviceOps, DeviceID, DeviceType,
		bus::pci,
		id::MajorBlock,
		manager::{DeviceManager, PhysicalDevice},
		storage::partition::read_partitions,
	},
	file::Mode,
	memory::{
		buddy::FrameOrder,
		cache::{FrameOwner, RcFrame},
		user::UserPtr,
	},
	syscall::{FromSyscallArg, ioctl},
};
use core::ffi::{c_uchar, c_ulong, c_ushort, c_void};
use partition::Partition;
use utils::{
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::{AllocResult, ENOMEM, EResult},
	ptr::arc::Arc,
};

/// The mode of the device file for a storage device
pub const STORAGE_MODE: Mode = 0o660;

/// Major number for SCSI devices
pub const SCSI_MAJOR: u32 = 8;

/// Hard drive geometry.
#[derive(Debug)]
#[repr(C)]
struct HdGeometry {
	/// The number of heads (CHS).
	heads: c_uchar,
	/// The number of sectors (CHS).
	sectors: c_uchar,
	/// The number of cylinders (CHS).
	cylinders: c_ushort,
	/// Starting LBA of the device.
	start: c_ulong,
}

/// Handle for the device file of a whole storage device or a partition.
#[derive(Debug)]
pub struct PartitionOps {
	/// The block device
	pub dev: Arc<BlkDev>,
	/// The partition associated with the handle. If `None`, the handle covers the whole device.
	pub partition: Partition,
}

impl BlockDeviceOps for PartitionOps {
	fn new_partition(&self, _dev: &BlkDev, _id: u32) -> AllocResult<(DeviceID, PathBuf)> {
		panic!("trying to create a partition of a partition");
	}

	fn read_frame(
		&self,
		_dev: &BlkDev,
		off: u64,
		order: FrameOrder,
		owner: FrameOwner,
	) -> EResult<RcFrame> {
		if off < self.partition.size {
			BlkDev::read_frame(&self.dev, self.partition.offset + off, order, owner)
		} else {
			Err(errno!(EINVAL))
		}
	}

	fn write_pages(&self, _dev: &BlkDev, off: u64, buf: &[u8]) -> EResult<()> {
		if off < self.partition.size {
			self.dev
				.ops
				.write_pages(&self.dev, self.partition.offset + off, buf)
		} else {
			Err(errno!(EINVAL))
		}
	}

	fn ioctl(&self, dev: &BlkDev, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::HDIO_GETGEO => {
				// Translate from LBA to CHS
				let size = self.partition.size;
				let s = (size % c_uchar::MAX as u64) as _;
				let h = ((size - s as u64) / c_uchar::MAX as u64 % c_uchar::MAX as u64) as _;
				let c = ((size - s as u64) / c_uchar::MAX as u64 / c_uchar::MAX as u64) as _;
				// Write to userspace
				let hd_geo_ptr = UserPtr::<HdGeometry>::from_ptr(argp as usize);
				hd_geo_ptr.copy_to_user(&HdGeometry {
					heads: h,
					sectors: s,
					cylinders: c,
					start: self.partition.offset as _,
				})?;
				Ok(0)
			}
			ioctl::BLKRRPART => {
				read_partitions(&self.dev)?;
				Ok(0)
			}
			ioctl::BLKSSZGET => {
				let blk_size = dev.blk_size.get();
				let size_ptr = UserPtr::<u32>::from_ptr(argp as usize);
				size_ptr.copy_to_user(&(blk_size as _))?;
				Ok(0)
			}
			ioctl::BLKGETSIZE64 => {
				let size = dev.blk_size.get() * self.partition.size;
				let size_ptr = UserPtr::<u64>::from_ptr(argp as usize);
				size_ptr.copy_to_user(&size)?;
				Ok(0)
			}
			_ => Err(errno!(ENOTTY)),
		}
	}
}

/// Manages storage controllers, devices and their partitions.
pub struct StorageManager {
	/// Allocated device major number for NVMe controllers
	nvme_ctrlr_major: MajorBlock,

	/// The list of detected interfaces
	interfaces: Vec<Arc<BlkDev>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> EResult<Self> {
		Ok(Self {
			nvme_ctrlr_major: MajorBlock::new_dyn(DeviceType::Char)?,

			interfaces: Vec::new(),
		})
	}
}

impl DeviceManager for StorageManager {
	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()> {
		// Ignore non-storage devices
		if dev.get_class() != pci::CLASS_MASS_STORAGE_CONTROLLER {
			return Ok(());
		}
		match (dev.get_subclass(), dev.get_prog_if()) {
			// IDE
			(0x01, _) => {
				let _ide = ide::Controller::new(dev);
				// TODO figure out what to do with the controller
			}
			// NVM
			(0x08, 0x02) => {
				let _nvm = match nvme::Controller::new(dev) {
					Ok(n) => n,
					Err(e) if e.as_int() == ENOMEM => return Err(e),
					Err(_) => return Ok(()),
				};
				// TODO figure out what to do with the controller
			}
			_ => {}
		}
		Ok(())
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		todo!() // remove device
	}
}
