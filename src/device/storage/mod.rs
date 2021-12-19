//! This module implements storage drivers.

pub mod ide;
pub mod mbr;
pub mod pata;
pub mod ramdisk;

use core::cmp::min;
use core::ffi::c_void;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::DeviceType;
use crate::device::bus::pci;
use crate::device::id::MajorBlock;
use crate::device::id;
use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::storage::ide::IDEController;
use crate::device::storage::pata::PATAInterface;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::Mode;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::process::oom;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// The major number for storage devices.
const STORAGE_MAJOR: u32 = 8;
/// The mode of the device file for a storage device.
const STORAGE_MODE: Mode = 0o660;
/// The maximum number of partitions in a disk.
const MAX_PARTITIONS: u32 = 16;

/// Trait representing a storage interface. A storage block is the atomic unit for I/O access on
/// the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	/// This value must always stay the same.
	fn get_block_size(&self) -> u64;
	/// Returns the number of storage blocks.
	/// This value must always stay the same.
	fn get_blocks_count(&self) -> u64;

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno>;

	// Unit testing is done through ramdisk testing
	/// Reads bytes from storage at offset `offset`, writting the data to `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn read_bytes(&self, buf: &mut [u8], offset: u64) -> Result<usize, Errno> {
		let block_size = self.get_block_size();
		let blk_begin = offset / block_size;
		let blk_end = (offset + buf.len() as u64) / block_size;
		if blk_begin >= self.get_blocks_count() || blk_end >= self.get_blocks_count() {
			return Err(errno::EINVAL);
		}

		// TODO Alloc only if needed?
		let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;

		let mut i = 0;
		while i < buf.len() {
			let storage_i = offset + i as u64;
			let block_off = (storage_i as usize) / block_size as usize;
			let block_inner_off = (storage_i as usize) % block_size as usize;
			let block_aligned = block_inner_off == 0;

			if !block_aligned {
				self.read(tmp_buf.get_slice_mut(), block_off as _, 1)?;

				let diff = min(buf.len(), block_size as usize - block_inner_off);
				for j in 0..diff {
					buf[i + j] = tmp_buf[block_inner_off + j];
				}

				i += diff;
			} else {
				let remaining_bytes = buf.len() - i;
				let remaining_blocks = remaining_bytes / block_size as usize;

				if remaining_bytes >= block_size as usize {
					let slice_len = remaining_blocks * block_size as usize;
					self.read(&mut buf[i..(i + slice_len)], block_off as _,
						remaining_blocks as _)?;

					i += slice_len;
				} else {
					self.read(tmp_buf.get_slice_mut(), block_off as _, 1)?;
					for j in 0..remaining_bytes {
						buf[i + j] = tmp_buf[j];
					}

					i += remaining_bytes;
				}
			}
		}

		Ok(buf.len())
	}

	// Unit testing is done through ramdisk testing
	/// Writes bytes to storage at offset `offset`, reading the data from `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn write_bytes(&mut self, buf: &[u8], offset: u64) -> Result<usize, Errno> {
		let block_size = self.get_block_size();
		let blk_begin = offset / block_size;
		let blk_end = (offset + buf.len() as u64) / block_size;
		if blk_begin >= self.get_blocks_count() || blk_end >= self.get_blocks_count() {
			return Err(errno::EINVAL);
		}

		// TODO Alloc only if needed?
		let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;

		let mut i = 0;
		while i < buf.len() {
			let storage_i = offset + i as u64;
			let block_off = (storage_i as usize) / block_size as usize;
			let block_inner_off = (storage_i as usize) % block_size as usize;
			let block_aligned = block_inner_off == 0;

			if !block_aligned {
				self.read(tmp_buf.get_slice_mut(), block_off as _, 1)?;

				let diff = min(buf.len(), block_size as usize - block_inner_off);
				for j in 0..diff {
					tmp_buf[block_inner_off + j] = buf[i + j];
				}

				self.write(tmp_buf.get_slice(), block_off as _, 1)?;
				i += diff;
			} else {
				let remaining_bytes = buf.len() - i;
				let remaining_blocks = remaining_bytes / block_size as usize;

				if remaining_bytes >= block_size as usize {
					let slice_len = remaining_blocks * block_size as usize;
					self.write(&buf[i..(i + slice_len)], block_off as _, remaining_blocks as _)?;

					i += slice_len;
				} else {
					self.read(tmp_buf.get_slice_mut(), block_off as _, 1)?;
					for j in 0..remaining_bytes {
						tmp_buf[j] = buf[i + j];
					}

					self.write(tmp_buf.get_slice(), block_off as _, 1)?;
					i += remaining_bytes;
				}
			}
		}

		Ok(buf.len())
	}
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
		offset: u64,
		/// The number of sectors in the partition.
		size: u64,
	}

	impl Partition {
		/// Creates a new instance with the given partition offset `offset` and size `size`.
		pub fn new(offset: u64, size: u64) -> Self {
			Self {
				offset,
				size,
			}
		}

		/// Returns the offset of the first sector of the partition.
		#[inline]
		pub fn get_offset(&self) -> u64 {
			self.offset
		}

		/// Returns the number of sectors in the partition.
		#[inline]
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
	pub fn read(storage: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
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
	/// A reference to the storage interface.
	interface: *mut dyn StorageInterface, // TODO Use a weak ptr?

	/// The offset to the beginning of the partition in bytes.
	partition_offset: u64,
	/// The size of the partition in bytes.
	partition_size: u64,
}

impl StorageDeviceHandle {
	/// Creates a new instance for the given storage interface and the given partition number. If
	/// the partition number is `0`, the device file is linked to the entire device instead of a
	/// partition.
	/// `interface` is the storage interface.
	/// `partition_offset` is the offset to the beginning of the partition in bytes.
	/// `partition_size` is the size of the partition in bytes.
	pub fn new(interface: *mut dyn StorageInterface, partition_offset: u64,
		partition_size: u64) -> Self {
		Self {
			interface,

			partition_offset,
			partition_size,
		}
	}
}

// TODO Handle partition
impl DeviceHandle for StorageDeviceHandle {
	fn ioctl(&mut self, _request: u32, _argp: *const c_void) -> Result<u32, Errno> {
		// TODO
		Err(errno::EINVAL)
	}
}

impl IO for StorageDeviceHandle {
	fn get_size(&self) -> u64 {
		let interface = unsafe { // Safe because the pointer is valid
			&*self.interface
		};

		interface.get_block_size() * interface.get_blocks_count()
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		let interface = unsafe { // Safe because the pointer is valid
			&mut *self.interface
		};

		interface.read_bytes(buff, offset)
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<usize, Errno> {
		let interface = unsafe { // Safe because the pointer is valid
			&mut *self.interface
		};

		interface.write_bytes(buff, offset)
	}
}

/// An instance of StorageManager manages devices on a whole major number.
/// The manager has name `storage`.
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
			major_block: id::alloc_major(DeviceType::Block, Some(STORAGE_MAJOR))?,
			interfaces: Vec::new(),
		})
	}

	// TODO Handle the case where there is more devices that the number of devices that can be
	// handled in the range of minor numbers
	// TODO When failing, remove previously registered devices
	/// Adds a storage device.
	fn add(&mut self, mut storage: Box<dyn StorageInterface>) -> Result<(), Errno> {
		// The device files' major number
		let major = self.major_block.get_major();
		// The id of the storage interface in the manager's list
		let storage_id = self.interfaces.len() as u32;

		// The size of a block on the storage device
		let block_size = storage.get_block_size();

		// The prefix is the path of the main device file
		let mut prefix = String::from(b"/dev/sd")?;
		prefix.push(b'a' + (storage_id as u8))?; // TODO Handle if out of the alphabet

		// The path of the main device file
		let main_path = Path::from_str(prefix.as_bytes(), false)?;
		// The total size of the interface in bytes
		let total_size = block_size * storage.get_blocks_count();

		// Creating the main device file
		let main_handle = StorageDeviceHandle::new(storage.as_mut_ptr(), 0, total_size);
		let main_device = Device::new(major, storage_id * MAX_PARTITIONS, main_path, STORAGE_MODE,
			DeviceType::Block, main_handle)?;
		device::register_device(main_device)?;

		// Creating device files for every partitions (in the limit of MAX_PARTITIONS)
		let partitions = partition::read(storage.as_mut())?;
		let count = min(MAX_PARTITIONS as usize, partitions.len());
		for i in 0..count {
			let partition = &partitions[i];

			// Adding the partition number to the path
			let path_str = (prefix.failable_clone()? + String::from_number(i as _)?)?;
			let path = Path::from_str(path_str.as_bytes(), false)?;

			// Computing the partition's offset and size
			let off = partition.get_offset() * block_size;
			let size = partition.get_size() * block_size;

			// Creating the partition's device file
			let handle = StorageDeviceHandle::new(storage.as_mut_ptr(), off, size);
			let device = Device::new(major, storage_id * MAX_PARTITIONS + i as u32, path,
				STORAGE_MODE, DeviceType::Block, handle)?;
			device::register_device(device)?;
		}

		self.interfaces.push(storage)
	}

	// TODO Function to remove a device

	/// Fills a random buffer `buff` of size `size` with seed `seed`.
	/// The function returns the seed for the next block.
	#[cfg(config_debug_storagetest)]
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
			let interfaces_count = self.interfaces.len();

			for j in 0..interfaces_count {
				let interface = &mut self.interfaces[j];

				crate::print!("Processing iteration: {}/{}; device: {}/{}...",
					i + 1, iterations_count,
					j + 1, interfaces_count);

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
	fn get_name(&self) -> &str {
		"storage"
	}

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

	fn on_plug(&mut self, dev: &dyn PhysicalDevice) {
		// Ignoring non-storage devices
		if dev.get_class() != pci::CLASS_MASS_STORAGE_CONTROLLER {
			return;
		}

		match dev.get_subclass() {
			// IDE controller
			0x01 => {
				let ide = IDEController::new(dev);
				oom::wrap(|| {
					let mut interfaces = ide.detect_all()?;
					for _ in 0..interfaces.len() {
						self.add(interfaces.pop().unwrap())?;
					}

					Ok(())
				});
			}

			// TODO Handle other controller types

			_ => {},
		}
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
		todo!();
	}
}
