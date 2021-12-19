//! A ramdisk is a virtual storage device stored on the RAM. From the point of view of the
//! userspace, it works exactly the same.
//! Ramdisks are lazily allocated so they do not use much memory as long as they are not used.

use core::ffi::c_void;
use core::mem::ManuallyDrop;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::DeviceType;
use crate::device::id;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::util::IO;
use crate::util::container::string::String;
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

	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		if offset > self.get_blocks_count() || offset + size > self.get_blocks_count() {
			return Err(errno::EINVAL);
		}

		if !self.is_allocated() {
			for b in buf {
				*b = 0;
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
		if offset > self.get_blocks_count() || offset + size > self.get_blocks_count() {
			return Err(errno::EINVAL);
		}

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
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for RAMDiskHandle {
	fn get_size(&self) -> u64 {
		RAM_DISK_SIZE as _
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		self.disk.read_bytes(buff, offset)
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		self.disk.write_bytes(buff, offset)
	}
}

/// Creates every ramdisk instances.
pub fn create() -> Result<(), Errno> {
	// TODO Undo all on fail?
	let _major = ManuallyDrop::new(id::alloc_major(DeviceType::Block, Some(RAM_DISK_MAJOR))?);

	for i in 0..RAM_DISK_COUNT {
		let mut name = String::from(b"ram")?;
		name.push_str(&String::from_number(i as _)?)?;

		let mut path = Path::root();
		path.push(String::from(b"dev")?)?;
		path.push(name)?;

		device::register_device(Device::new(RAM_DISK_MAJOR, i as _, path, 0o666, DeviceType::Block,
			RAMDiskHandle::new())?)?;
	}

	Ok(())
}

/*#[cfg(test)]
mod test {
	use super::*;
	use core::cmp::min;

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
