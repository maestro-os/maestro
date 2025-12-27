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
mod partition;
mod pata;

use crate::{
	device,
	device::{
		BLK_DEVICES, BlkDev, BlockDeviceOps, DeviceID, DeviceType,
		bus::pci,
		id,
		id::MajorBlock,
		manager::{DeviceManager, PhysicalDevice},
	},
	file::Mode,
	memory::{
		buddy::FrameOrder,
		cache::{FrameOwner, RcFrame},
		user::UserPtr,
	},
	syscall::{FromSyscallArg, ioctl},
};
use core::{
	ffi::{c_uchar, c_ulong, c_ushort, c_void},
	num::NonZeroU64,
};
use partition::Partition;
use utils::{
	TryClone,
	boxed::Box,
	collections::{
		path::{Path, PathBuf},
		vec::Vec,
	},
	errno,
	errno::EResult,
	format,
	ptr::arc::Arc,
};

/// The major number for storage devices.
const SCSI_MAJOR: u32 = 8;

/// NVMe controller major number
const NVME_CONTROLLER_MAJOR: u32 = 239;
/// NVME driver major number
const NVME_MAJOR: u32 = 259;

/// The mode of the device file for a storage device.
const STORAGE_MODE: Mode = 0o660;
/// The maximum number of partitions in a disk.
const MAX_PARTITIONS: usize = 16;

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

	/// The ID of the storage device in the manager.
	pub storage_id: u32,
	/// The path to the file of the main device containing the partition table.
	pub path_prefix: PathBuf,
}

impl BlockDeviceOps for PartitionOps {
	fn block_size(&self) -> NonZeroU64 {
		self.dev.ops.block_size()
	}

	fn blocks_count(&self) -> u64 {
		self.dev.ops.blocks_count()
	}

	fn read_frame(&self, off: u64, order: FrameOrder, owner: FrameOwner) -> EResult<RcFrame> {
		if off < self.partition.size {
			BlkDev::read_frame(&self.dev, self.partition.offset + off, order, owner)
		} else {
			Err(errno!(EINVAL))
		}
	}

	fn write_pages(&self, off: u64, buf: &[u8]) -> EResult<()> {
		if off < self.partition.size {
			self.dev.ops.write_pages(self.partition.offset + off, buf)
		} else {
			Err(errno!(EINVAL))
		}
	}

	fn ioctl(&self, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
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
				StorageManager::clear_partitions(self.dev.id.major)?;
				StorageManager::read_partitions(
					self.dev.clone(),
					self.storage_id,
					&self.path_prefix,
				)?;
				Ok(0)
			}
			ioctl::BLKSSZGET => {
				let blk_size = self.block_size();
				let size_ptr = UserPtr::<u32>::from_ptr(argp as usize);
				size_ptr.copy_to_user(&(blk_size.get() as _))?;
				Ok(0)
			}
			ioctl::BLKGETSIZE64 => {
				let size = self.block_size().get() * self.blocks_count();
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
	/// Allocated device major number for SCSI devices
	scsi_major: MajorBlock,
	/// Allocated device major number for NVMe controllers
	nvme_ctrlr_major: MajorBlock,
	/// Allocated device major number for NVMe drives
	nvme_major: MajorBlock,

	/// The list of detected interfaces
	interfaces: Vec<Arc<BlkDev>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> EResult<Self> {
		Ok(Self {
			scsi_major: id::alloc_major(DeviceType::Block, Some(SCSI_MAJOR))?,
			nvme_ctrlr_major: id::alloc_major(DeviceType::Char, Some(NVME_CONTROLLER_MAJOR))?,
			nvme_major: id::alloc_major(DeviceType::Block, Some(NVME_MAJOR))?,

			interfaces: Vec::new(),
		})
	}

	/// Creates device files for every partitions on the storage device, within the limit of
	/// `MAX_PARTITIONS`.
	///
	/// Arguments:
	/// - `dev` is the block device
	/// - `storage_id` is the ID of the storage device in the manager
	/// - `path_prefix` is the path to the file of the main device containing the partition table
	pub fn read_partitions(dev: Arc<BlkDev>, storage_id: u32, path_prefix: &Path) -> EResult<()> {
		let Some(partitions_table) = partition::read(&dev)? else {
			return Ok(());
		};
		let partitions = partitions_table.read_partitions(&dev)?;

		// TODO When failing, remove previously registered devices
		let iter = partitions.into_iter().take(MAX_PARTITIONS - 1).enumerate();
		for (i, partition) in iter {
			let part_nbr = (i + 1) as u32;
			let path = PathBuf::try_from(format!("{path_prefix}{part_nbr}")?)?;

			// Create the partition's device file
			let handle = Box::new(PartitionOps {
				dev: dev.clone(),
				partition,

				storage_id,
				path_prefix: path_prefix.to_path_buf()?,
			})?;
			let dev = BlkDev::new(
				DeviceID {
					// TODO use a different major for different storage device types
					major: SCSI_MAJOR,
					minor: storage_id * MAX_PARTITIONS as u32 + part_nbr,
				},
				path,
				STORAGE_MODE,
				handle,
			)?;
			device::register_blk(dev)?;
		}

		Ok(())
	}

	/// Clears device files for every partition.
	///
	/// `major` is the major number of the devices to be removed.
	pub fn clear_partitions(major: u32) -> EResult<()> {
		let mut blk_devices = BLK_DEVICES.lock();
		for i in 1..MAX_PARTITIONS {
			blk_devices.remove(&DeviceID {
				major,
				minor: i as _,
			});
		}
		Ok(())
	}

	// TODO Handle the case where there is more devices that the number of devices
	// that can be handled in the range of minor numbers
	// TODO When failing, remove previously registered devices
	/// Registers a new storage device.
	fn add(&mut self, ops: Box<dyn BlockDeviceOps>) -> EResult<()> {
		let storage_id = self.interfaces.len() as u32;
		// Prefix is the path of the main device file
		// TODO Handle if out of the alphabet
		let letter = (b'a' + storage_id as u8) as char;
		let main_path = PathBuf::try_from(format!("/dev/sd{letter}")?)?;
		// Create the main device file
		let dev = BlkDev::new(
			DeviceID {
				major: self.scsi_major.get_major(),
				minor: storage_id * MAX_PARTITIONS as u32,
			},
			main_path.try_clone()?,
			STORAGE_MODE,
			ops,
		)?;
		device::register_blk(dev.clone())?;
		Self::read_partitions(dev.clone(), storage_id, &main_path)?;
		self.interfaces.push(dev)?;
		Ok(())
	}

	// TODO Function to remove a device

	/// Fills a random buffer `buff` of size `size` with seed `seed`.
	///
	/// The function returns the seed for the next block.
	#[cfg(config_debug_storage_test)]
	fn random_block(size: u64, buff: &mut [u8], seed: u32) -> u32 {
		let mut s = seed;

		for i in 0..size {
			s = crate::util::math::pseudo_rand(s, 1664525, 1013904223, 0x100);
			buff[i as usize] = (s & 0xff) as u8;
		}

		s
	}

	// TODO Test with several blocks at a time
	/// Tests the given interface with the given interface `interface`.
	///
	/// `seed` is the seed for pseudo random generation. The function will set
	/// this variable to another value for the next iteration.
	#[cfg(config_debug_storage_test)]
	fn test_interface(interface: &mut dyn StorageInterface, seed: u32) -> bool {
		let block_size = interface.get_block_size();
		let blocks_count = min(1024, interface.get_blocks_count());

		let mut s = seed;
		for i in 0..blocks_count {
			let mut buff: [u8; 512] = [0; 512]; // TODO Set to block size
			s = Self::random_block(block_size, &mut buff, s);
			if interface.write(&buff, i, 1).is_err() {
				crate::println!("\nCannot write to disk on block {}.", i);
				return false;
			}
		}

		s = seed;
		for i in 0..blocks_count {
			let mut buff: [u8; 512] = [0; 512]; // TODO Set to block size
			s = Self::random_block(interface.get_block_size(), &mut buff, s);

			let mut buf: [u8; 512] = [0; 512]; // TODO Set to block size
			if interface.read(&mut buf, i, 1).is_err() {
				crate::println!("\nCannot read from disk on block {}.", i);
				return false;
			}

			if buf != buff {
				return false;
			}
		}

		true
	}

	/// Performs testing of storage devices and drivers.
	///
	/// If every tests pass, the function returns `true`. Else, it returns
	/// `false`.
	#[cfg(config_debug_storage_test)]
	fn perform_test(&mut self) -> bool {
		let mut seed = 42;
		let iterations_count = 10;
		for i in 0..iterations_count {
			let interfaces_count = self.interfaces.len();

			for j in 0..interfaces_count {
				let mut interface = self.interfaces[j].lock();

				crate::print!(
					"Processing iteration: {}/{iterations_count}; device: {}/{iterations_count}...",
					i + 1,
					j + 1,
				);

				if !Self::test_interface(&mut *interface, seed) {
					return false;
				}

				seed = crate::util::math::pseudo_rand(seed, 1103515245, 12345, 0x100);
			}

			if i < iterations_count - 1 {
				crate::print!("\r");
			} else {
				crate::println!();
			}
		}

		true
	}

	/// Tests every storage drivers on every storage devices.
	///
	/// The execution of this function removes all the data on every connected
	/// writable disks, so it must be used carefully.
	#[cfg(config_debug_storage_test)]
	pub fn test(&mut self) {
		crate::println!("Running disks tests... ({} devices)", self.interfaces.len());

		if self.perform_test() {
			crate::println!("Done!");
		} else {
			crate::println!("Storage test failed!");
		}
		crate::halt();
	}
}

impl DeviceManager for StorageManager {
	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()> {
		// Ignore non-storage devices
		if dev.get_class() != pci::CLASS_MASS_STORAGE_CONTROLLER {
			return Ok(());
		}
		match dev.get_subclass() {
			// IDE
			0x01 => {
				let ide = ide::Controller::new(dev);
				for iface in ide.detect() {
					self.add(iface?)?;
				}
			}
			// NVM
			0x08 => {
				let Some(_nvm) = nvme::Controller::new(dev) else {
					return Ok(());
				};
				// TODO insert controller device
				// TODO insert drives devices
			}
			_ => {}
		}
		Ok(())
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		todo!() // remove device
	}
}
