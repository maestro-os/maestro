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

/// The ramdisks' major number.
const RAM_DISK_MAJOR: u32 = 1;
/// The number of ramdisks on the system.
const RAM_DISK_COUNT: usize = 16;
/// The size of the ramdisk in bytes.
const RAM_DISK_SIZE: usize = 4 * 1024 * 1024;

// TODO Add a mechanism to swap on the disk?
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
			self.data = Some(malloc::Alloc::new(RAM_DISK_SIZE)?);
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
	fn read(&mut self, _offset: usize, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO

		Ok(0)
	}

	fn write(&mut self, _offset: usize, _buff: &[u8]) -> Result<usize, Errno> {
		// TODO

		Ok(0)
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

		let mut device = Device::new(RAM_DISK_MAJOR, i as _, path, 0666, DeviceType::Block,
			RAMDiskHandle::new())?;
		device.create_file()?;
		device::register_device(device)?;
	}

	Ok(())
}
