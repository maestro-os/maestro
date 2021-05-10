/// This module implements storage drivers.

pub mod mbr;
pub mod pata;
pub mod ramdisk;

use core::cmp::min;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::DeviceType;
use crate::device::id::MajorBlock;
use crate::device::id;
use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::storage::pata::PATAInterface;
use crate::device;
use crate::errno::Errno;
use crate::file::Mode;
use crate::file::path::Path;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// The major number for storage devices.
const STORAGE_MAJOR: u32 = 8;
/// The mode of the device file for a storage device.
const STORAGE_MODE: Mode = 0660;
/// The maximum number of partitions in a disk.
const MAX_PARTITIONS: u32 = 16;

/// Trait representing a storage interface. A storage block is the atomic unit for I/O access on
/// the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	fn get_block_size(&self) -> usize;
	/// Returns the number of storage blocks.
	fn get_blocks_count(&self) -> u64;

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), ()>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	fn write(&self, buf: &[u8], offset: u64, size: u64) -> Result<(), ()>;
}

pub mod partition {
	use crate::errno::Errno;
	use crate::errno;
	use crate::util::container::vec::Vec;
	use super::StorageInterface;
	use super::mbr::MBRTable;

	/// Structure representing a disk partition.
	pub struct Partition {
		/// The offset to the first sector of the partition.
		start: u64,
		/// The number of sectors in the partition.
		size: u64,
	}

	impl Partition {
		/// Creates a new instance with the given start partition `start` and size `size`.
		pub fn new(start: u64, size: u64) -> Self {
			Self {
				start: start,
				size: size,
			}
		}

		/// Returns the offset of the first sector of the partition.
		pub fn get_start(&self) -> u64 {
			self.start
		}

		/// Returns the number fo sectors in the partition.
		pub fn get_size(&self) -> u64 {
			self.size
		}
	}

	/// Trait representing a partition table.
	pub trait Table {
		/// Returns the type of the partition table.
		fn get_type(&self) -> &'static str;

		/// Reads the partitions list.
		fn read(&self) -> Result<Vec<Partition>, Errno>;
	}

	/// Reads the list of partitions from the given storage interface `storage`.
	pub fn read(storage: &dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
		if storage.get_block_size() != 512 {
			return Ok(Vec::new());
		}

		let mut first_sector: [u8; 512] = [0; 512];
		if storage.read(&mut first_sector, 0, 1).is_err() {
			return Err(errno::EIO);
		}

		// Valid because taking the pointer to the buffer on the stack which has the same size as
		// the structure
		let mbr_table = unsafe {
			&*(first_sector.as_ptr() as *const MBRTable)
		};
		if mbr_table.is_valid() {
			return mbr_table.read();
		}

		// TODO Try to detect GPT

		Ok(Vec::new())
	}
}

/// Handle for the device file of a storage device or a storage device partition.
pub struct StorageDeviceHandle {
	/// The id of the device in the storage manager's list.
	id: u32,
	/// The if of the partition to be handled. If 0, the device handles the whole device.
	partition: u32,

	/// The reference to the storage manager.
	storage_manager: *mut StorageManager, // TODO Use a weak ptr?
}

impl StorageDeviceHandle {
	/// Creates a new instance for the given storage device with identifier `id` and the given
	/// partition number `partition. If the partition number is `0`, the device file is linked to
	/// the entire device instead of a partition.
	/// `storage_manager` is the storage manager associated with the device.
	pub fn new(id: u32, partition: u32, storage_manager: *mut StorageManager) -> Self {
		Self {
			id: id,
			partition: partition,

			storage_manager: storage_manager,
		}
	}
}

impl DeviceHandle for StorageDeviceHandle {
	fn read(&mut self, _offset: usize, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO
		Ok(0)
	}

	fn write(&mut self, _offset: usize, _buff: &[u8]) -> Result<usize, Errno> {
		// TODO
		Ok(0)
	}
}

/// Structure managing storage devices.
pub struct StorageManager {
	/// The allocated device major number for storage devices.
	major_block: MajorBlock,
	/// The list of detected interfaces.
	interfaces: Vec<Box<dyn StorageInterface>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			major_block: id::alloc_major(Some(STORAGE_MAJOR))?,
			interfaces: Vec::new(),
		})
	}

	// TODO Handle the case where there is more devices that the number of devices that can be
	// handled in the range of minor numbers
	// TODO When failing, remove previously registered devices
	/// Adds a storage device.
	fn add(&mut self, storage: Box<dyn StorageInterface>) -> Result<(), Errno> {
		let major = self.major_block.get_major();
		let storage_id = self.interfaces.len() as u32;

		let mut prefix = String::from("/dev/sd")?;
		prefix.push(unsafe { // Safe because the id stays in range of the alphabet
			char::from_u32_unchecked((('a' as u8) + (storage_id as u8)) as _)
		})?;

		let device_type = if storage.get_block_size() == 1 {
			DeviceType::Char
		} else {
			DeviceType::Block
		};

		let main_path = Path::from_string(prefix.as_str())?;
		let main_handle = StorageDeviceHandle::new(storage_id, 0, self);
		let main_device = Device::new(major, storage_id * MAX_PARTITIONS, main_path, STORAGE_MODE,
			device_type, main_handle)?;
		device::register_device(main_device)?;

		let partitions = partition::read(storage.as_ref())?;
		for i in 0..min(MAX_PARTITIONS, partitions.len() as u32) {
			let path = Path::from_string(prefix.as_str())?; // TODO Add i + 1 as char at the end
			let handle = StorageDeviceHandle::new(storage_id, i, self);
			let device = Device::new(major, storage_id * MAX_PARTITIONS + i, path, STORAGE_MODE,
				device_type, handle)?;
			device::register_device(device)?;
		}

		self.interfaces.push(storage)
	}

	// TODO Function to remove a device

	/// Fills a random buffer `buff` of size `size` with seed `seed`.
	/// The function returns the seed for the next block.
	#[cfg(config_debug_storagetest)]
	fn random_block(size: usize, buff: &mut [u8], seed: u32) -> u32 {
		let mut s = seed;

		for i in 0..size {
			s = crate::util::math::pseudo_rand(s, 1664525, 1013904223, 0x100);
			buff[i] = (s & 0xff) as u8;
		}

		s
	}

	// TODO Test with several blocks at a time
	/// Tests the given interface with the given interface `interface`.
	/// `seed` is the seed for pseudo random generation. The function will set this variable to
	/// another value for the next iteration.
	#[cfg(config_debug_storagetest)]
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
	/// If every tests pass, the function returns `true`. Else, it returns `false`.
	#[cfg(config_debug_storagetest)]
	fn perform_test(&mut self) -> bool {
		let mut seed = 42;
		let iterations_count = 10;
		for i in 0..iterations_count {
			for (j, interface) in self.interfaces.iter_mut().enumerate() {
				crate::print!("Processing iteration: {}/{}; device: {}/{}...",
					i + 1, iterations_count,
					j + 1, self.interfaces.len());

				if !Self::test_interface(interface.as_mut(), seed) {
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
	/// The execution of this function removes all the data on every connected writable disks, so
	/// it must be used carefully.
	#[cfg(config_debug_storagetest)]
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
	fn legacy_detect(&mut self) -> Result<(), Errno> {
		// TODO Detect floppy disks

		for i in 0..4 {
			let secondary = (i & 0b10) != 0;
			let slave = (i & 0b01) != 0;

			if let Ok(dev) = PATAInterface::new(secondary, slave) {
				self.add(Box::new(dev)?)?;
			}
		}

		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}
