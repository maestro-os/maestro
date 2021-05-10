/// A ramdisk is a virtual storage device stored on the RAM. From the point of view of the
/// userspace, it works exactly the same.
/// Ramdisks are lazily allocated so they do not use much memory as long as they are not used.

use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::DeviceType;
use crate::device;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::util::container::string::String;
use crate::util::math;
use super::StorageInterface;

/// The ramdisks' major number.
const RAM_DISK_MAJOR: u32 = 1;
/// The number of ramdisks on the system.
const RAM_DISK_COUNT: usize = 16;
/// The size of the ramdisk in bytes.
const RAM_DISK_SIZE: usize = 4 * 1024 * 1024;

// TODO Add a mechanism to free when cleared?

/// Structure representing a ram disk.
struct RAMDisk {
	/// The ram's data.
	data: Option<malloc::Alloc<u8>>,
}

impl RAMDisk {
	/// Creates a new ramdisk.
	pub fn new() -> Self {
		Self {
			data: None,
		}
	}

	/// Tells whether the disk is allocated.
	pub fn is_allocated(&self) -> bool {
		self.data.is_some()
	}

	/// If not allocated, allocates the disk.
	pub fn allocate(&mut self) -> Result<(), Errno> {
		if self.data.is_none() {
			self.data = Some(malloc::Alloc::new_default(RAM_DISK_SIZE)?);
		}

		Ok(())
	}
}

impl StorageInterface for RAMDisk {
	fn get_block_size(&self) -> u64 {
		512
	}

	fn get_blocks_count(&self) -> u64 {
		(RAM_DISK_SIZE as u64) / self.get_block_size()
	}

	fn read(&mut self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		if !self.is_allocated() {
			for i in 0..buf.len() {
				buf[i] = 0;
			}
		} else {
			let block_size = self.get_block_size();
			let off = offset * block_size;

			for i in 0..size {
				for j in 0..block_size {
					let buf_index = (i * block_size + j) as usize;
					let disk_index = (off + buf_index as u64) as usize;

					buf[buf_index] = self.data.as_ref().unwrap()[disk_index];
				}
			}
		}

		Ok(())
	}

	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno> {
		self.allocate()?;

		let block_size = self.get_block_size();
		let off = offset * block_size;

		for i in 0..size {
			for j in 0..block_size {
				let buf_index = (i * block_size + j) as usize;
				let disk_index = (off + buf_index as u64) as usize;

				self.data.as_mut().unwrap()[disk_index] = buf[buf_index];
			}
		}

		Ok(())
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

impl DeviceHandle for RAMDiskHandle {
	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		let block_off = offset / self.disk.get_block_size();
		let blocks_count = math::ceil_division(buff.len() as u64, self.disk.get_block_size());

		self.disk.read(buff, block_off, blocks_count)?;

		Ok(buff.len())
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		let block_size = self.disk.get_block_size();
		let block_off = offset / block_size;
		let blocks_count = math::ceil_division(buff.len() as u64, block_size);

		// TODO Read first and last sectors to complete them
		//let begin_inner_off = offset % block_size;
		//if begin_inner_off != 0 {
			// TODO
		//}

		self.disk.write(buff, block_off, blocks_count)?;

		Ok(buff.len())
	}
}

/// Creates every ramdisk instances.
pub fn create() -> Result<(), Errno> {
	// TODO Undo all on fail?
	// TODO Alloc major number block

	for i in 0..RAM_DISK_COUNT {
		let mut name = String::from("name")?;
		name.push_str(&String::from_number(i as _)?)?;

		let mut path = Path::root();
		path.push(String::from("/dev")?)?;
		path.push(name)?;

		device::register_device(Device::new(RAM_DISK_MAJOR, i as _, path, 0666, DeviceType::Block,
			RAMDiskHandle::new())?)?;
	}

	Ok(())
}
