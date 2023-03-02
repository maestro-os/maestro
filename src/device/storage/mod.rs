//! This module implements storage drivers.

pub mod cache;
pub mod ide;
pub mod partition;
pub mod pata;
pub mod ramdisk;

use core::cmp::min;
use core::ffi::c_uchar;
use core::ffi::c_ulong;
use core::ffi::c_ushort;
use core::ffi::c_void;
use crate::device::Device;
use crate::device::DeviceHandle;
use crate::device::DeviceID;
use crate::device::DeviceType;
use crate::device::bus::pci;
use crate::device::id::MajorBlock;
use crate::device::id;
use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::Mode;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::oom;
use crate::syscall::ioctl;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::math;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;
use crate::util::ptr::WeakPtr;
use partition::Partition;

/// The major number for storage devices.
const STORAGE_MAJOR: u32 = 8;
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

/// Trait representing a storage interface. A storage block is the atomic unit
/// for I/O access on the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	/// This value must not change.
	fn get_block_size(&self) -> u64;
	/// Returns the number of storage blocks.
	/// This value must not change.
	fn get_blocks_count(&self) -> u64;

	/// Returns the size of the storage in bytes.
	/// This value must not change.
	fn get_size(&self) -> u64 {
		self.get_block_size() * self.get_blocks_count()
	}

	/// Reads `size` blocks from storage at block offset `offset`, writing the
	/// data to `buf`. If the offset and size are out of bounds, the function
	/// returns an error.
	fn read(&mut self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the
	/// data from `buf`. If the offset and size are out of bounds, the function
	/// returns an error.
	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno>;

	// Unit testing is done through ramdisk testing
	/// Reads bytes from storage at offset `offset`, writing the data to `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn read_bytes(&mut self, buf: &mut [u8], offset: u64) -> Result<(u64, bool), Errno> {
		let block_size = self.get_block_size();
		let blocks_count = self.get_blocks_count();

		let blk_begin = offset / block_size;
		let blk_end = math::ceil_division(offset + buf.len() as u64, block_size);
		if blk_begin > blocks_count || blk_end > blocks_count {
			return Err(errno!(EINVAL));
		}

		let mut i = 0;
		while i < buf.len() {
			let remaining_bytes = buf.len() - i;

			let storage_i = offset + i as u64;
			let block_off = storage_i / block_size;
			let block_inner_off = (storage_i as usize) % block_size as usize;
			let block_aligned = block_inner_off == 0;

			if !block_aligned {
				let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;
				self.read(tmp_buf.as_slice_mut(), block_off, 1)?;

				let diff = min(remaining_bytes, block_size as usize - block_inner_off);
				for j in 0..diff {
					debug_assert!(i + j < buf.len());
					debug_assert!(block_inner_off + j < tmp_buf.len());
					buf[i + j] = tmp_buf[block_inner_off + j];
				}

				i += diff;
			} else if (remaining_bytes as u64) < block_size {
				let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;
				self.read(tmp_buf.as_slice_mut(), block_off, 1)?;

				for j in 0..remaining_bytes {
					debug_assert!(i + j < buf.len());
					debug_assert!(j < tmp_buf.len());
					buf[i + j] = tmp_buf[j];
				}

				i += remaining_bytes;
			} else {
				let remaining_blocks = (remaining_bytes as u64) / block_size;
				let len = (remaining_blocks * block_size) as usize;
				debug_assert!(i + len <= buf.len());
				self.read(&mut buf[i..(i + len)], block_off, remaining_blocks as _)?;

				i += len;
			}
		}

		let eof = (offset + buf.len() as u64) >= block_size * blocks_count;
		Ok((buf.len() as _, eof))
	}

	// Unit testing is done through ramdisk testing
	/// Writes bytes to storage at offset `offset`, reading the data from `buf`.
	/// If the offset and size are out of bounds, the function returns an error.
	fn write_bytes(&mut self, buf: &[u8], offset: u64) -> Result<u64, Errno> {
		let block_size = self.get_block_size();
		let blocks_count = self.get_blocks_count();

		let blk_begin = offset / block_size;
		let blk_end = math::ceil_division(offset + buf.len() as u64, block_size);
		if blk_begin > blocks_count || blk_end > blocks_count {
			return Err(errno!(EINVAL));
		}

		let mut i = 0;
		while i < buf.len() {
			let remaining_bytes = buf.len() - i;

			let storage_i = offset + i as u64;
			let block_off = storage_i / block_size;
			let block_inner_off = (storage_i as usize) % block_size as usize;
			let block_aligned = block_inner_off == 0;

			if !block_aligned {
				let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;
				self.read(tmp_buf.as_slice_mut(), block_off, 1)?;

				let diff = min(remaining_bytes, block_size as usize - block_inner_off);
				for j in 0..diff {
					debug_assert!(i + j < buf.len());
					debug_assert!(block_inner_off + j < tmp_buf.len());
					tmp_buf[block_inner_off + j] = buf[i + j];
				}

				self.write(tmp_buf.as_slice(), block_off, 1)?;

				i += diff;
			} else if (remaining_bytes as u64) < block_size {
				let mut tmp_buf = malloc::Alloc::<u8>::new_default(block_size as _)?;
				self.read(tmp_buf.as_slice_mut(), block_off, 1)?;

				for j in 0..remaining_bytes {
					debug_assert!(i + j < buf.len());
					debug_assert!(j < tmp_buf.len());
					tmp_buf[j] = buf[i + j];
				}

				self.write(tmp_buf.as_slice(), block_off, 1)?;

				i += remaining_bytes;
			} else {
				let remaining_blocks = (remaining_bytes as u64) / block_size;
				let len = (remaining_blocks * block_size) as usize;
				debug_assert!(i + len <= buf.len());
				self.write(&buf[i..(i + len)], block_off, remaining_blocks as _)?;

				i += len;
			}
		}

		Ok(buf.len() as _)
	}
}

/// Handle for the device file of a whole storage device or a partition.
pub struct StorageDeviceHandle {
	/// A reference to the storage interface.
	interface: WeakPtr<dyn StorageInterface>,
	/// The partition associated with the handle. If `None`, the handle covers the whole device.
	partition: Option<Partition>,

	/// The major number of the device.
	major: u32,
	/// The ID of the storage device in the manager.
	storage_id: u32,
	/// The path to the file of the main device containing the partition table.
	path_prefix: String,
}

impl StorageDeviceHandle {
	/// Creates a new instance for the given storage interface and the given
	/// partition number.
	///
	/// Arguments:
	/// - `interface` is the storage interface.
	/// - `partition` is the partition. If `None`, the handle works on the whole storage device.
	/// - `major` is the major number of the device.
	/// - `storage_id` is the ID of the storage device in the manager.
	/// - `path_prefix` is the path to the file of the main device containing the partition table.
	pub fn new(
		interface: WeakPtr<dyn StorageInterface>,
		partition: Option<Partition>,
		major: u32,
		storage_id: u32,
		path_prefix: String
	) -> Self {
		Self {
			interface,
			partition,

			major,
			storage_id,
			path_prefix
		}
	}
}

impl DeviceHandle for StorageDeviceHandle {
	fn ioctl(
		&mut self,
		mem_space: IntSharedPtr<MemSpace>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> Result<u32, Errno> {
		match request.get_old_format() {
			ioctl::HDIO_GETGEO => {
				// The total size of the disk
				let size = {
					if let Some(interface) = self.interface.get() {
						let interface_guard = interface.lock();
						let interface = interface_guard.get();

						interface.get_block_size() * interface.get_blocks_count()
					} else {
						0
					}
				};

				// Translate from LBA to CHS
				let s = (size % c_uchar::MAX as u64) as _;
				let h = ((size - s as u64) / c_uchar::MAX as u64 % c_uchar::MAX as u64) as _;
				let c = ((size - s as u64) / c_uchar::MAX as u64 / c_uchar::MAX as u64) as _;

				// Starting LBA of the partition
				let start = self.partition.as_ref()
					.map(|p| p.get_offset())
					.unwrap_or(0) as _;

				let hd_geo = HdGeometry {
					heads: h,
					sectors: s,
					cylinders: c,
					start,
				};

				// Write to userspace
				let mem_space_guard = mem_space.lock();
				let hd_geo_ptr: SyscallPtr<HdGeometry> = (argp as usize).into();
				let hd_geo_ref = hd_geo_ptr
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*hd_geo_ref = hd_geo;

				Ok(0)
			}

			ioctl::BLKRRPART => {
				StorageManager::clear_partitions(self.major)?;
				StorageManager::read_partitions(
					self.interface.clone(),
					self.major,
					self.storage_id,
					self.path_prefix.failable_clone()?
				)?;

				Ok(0)
			}

			ioctl::BLKSSZGET => {
				let blk_size = {
					if let Some(interface) = self.interface.get() {
						let interface_guard = interface.lock();
						let interface = interface_guard.get();

						interface.get_block_size()
					} else {
						0
					}
				};

				let mem_space_guard = mem_space.lock();
				let size_ptr: SyscallPtr<u64> = (argp as usize).into();
				let size_ref = size_ptr
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*size_ref = blk_size;

				Ok(0)
			}

			ioctl::BLKGETSIZE64 => {
				let size = self.get_size();

				let mem_space_guard = mem_space.lock();
				let size_ptr: SyscallPtr<u64> = (argp as usize).into();
				let size_ref = size_ptr
					.get_mut(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*size_ref = size;

				Ok(0)
			}

			_ => Err(errno!(EINVAL)),
		}
	}
}

impl IO for StorageDeviceHandle {
	fn get_size(&self) -> u64 {
		if let Some(interface) = self.interface.get() {
			let interface_guard = interface.lock();
			let interface = interface_guard.get();

			let blocks_count = self.partition.as_ref()
				.map(|p| p.get_size())
				.unwrap_or_else(|| interface.get_blocks_count());
			interface.get_block_size() * blocks_count
		} else {
			0
		}
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if let Some(interface) = self.interface.get() {
			let interface_guard = interface.lock();
			let interface = interface_guard.get_mut();

			// Check offset
			let (start, size) = match &self.partition {
				Some(p) => {
					let start = p.get_offset() * interface.get_block_size();
					let size = p.get_size() * interface.get_block_size();

					(start, size)
				}

				None => (0, interface.get_size()),
			};
			if (offset + buff.len() as u64) > size {
				return Err(errno!(EINVAL));
			}

			interface.read_bytes(buff, start + offset)
		} else {
			Err(errno!(ENODEV))
		}
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		if let Some(interface) = self.interface.get() {
			let interface_guard = interface.lock();
			let interface = interface_guard.get_mut();

			// Check offset
			let (start, size) = match &self.partition {
				Some(p) => {
					let start = p.get_offset() * interface.get_block_size();
					let size = p.get_size() * interface.get_block_size();

					(start, size)
				}

				None => (0, interface.get_size()),
			};
			if (offset + buff.len() as u64) > size {
				return Err(errno!(EINVAL));
			}

			interface.write_bytes(buff, start + offset)
		} else {
			Err(errno!(ENODEV))
		}
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Ok(0)
	}
}

/// An instance of StorageManager manages devices on a whole major number.
/// The manager has name `storage`.
pub struct StorageManager {
	/// The allocated device major number for storage devices.
	major_block: MajorBlock,
	/// The list of detected interfaces.
	interfaces: Vec<SharedPtr<dyn StorageInterface>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			major_block: id::alloc_major(DeviceType::Block, Some(STORAGE_MAJOR))?,
			interfaces: Vec::new(),
		})
	}

	// TODO When failing, remove previously registered devices
	/// Creates device files for every partitions on the storage device, within the limit of
	/// `MAX_PARTITIONS`.
	///
	/// Arguments:
	/// - `storage` is the storage interface.
	/// - `major` is the major number of the device.
	/// - `storage_id` is the ID of the storage device in the manager.
	/// - `path_prefix` is the path to the file of the main device containing the partition table.
	pub fn read_partitions(
		storage: WeakPtr<dyn StorageInterface>,
		major: u32,
		storage_id: u32,
		path_prefix: String,
	) -> Result<(), Errno> {
		if let Some(storage_mutex) = storage.get() {
			let storage_guard = storage_mutex.lock();
			let s = storage_guard.get_mut();

			if let Some(partitions_table) = partition::read(s)? {
				let partitions = partitions_table.get_partitions(s)?;

				let iter = partitions.into_iter().take(MAX_PARTITIONS - 1);
				for (i, partition) in iter.enumerate() {
					let part_nbr = (i + 1) as u32;

					// Adding the partition number to the path
					let path_str = (
						path_prefix.failable_clone()? + crate::format!("{}", part_nbr)?
					)?;
					let path = Path::from_str(path_str.as_bytes(), false)?;

					// Creating the partition's device file
					let handle = StorageDeviceHandle::new(
						storage.clone(),
						Some(partition),
						major,
						storage_id,
						path_prefix.failable_clone()?
					);
					let device = Device::new(
						DeviceID {
							type_: DeviceType::Block,
							// TODO use a different major for different storage device types
							major: STORAGE_MAJOR,
							minor: storage_id * MAX_PARTITIONS as u32 + part_nbr,
						},
						path,
						STORAGE_MODE,
						handle,
					)?;
					device::register(device)?;
				}
			}
		}

		Ok(())
	}

	/// Clears device files for every partitions.
	///
	/// `major` is the major number of the devices to be removed.
	pub fn clear_partitions(major: u32) -> Result<(), Errno> {
		for i in 1..MAX_PARTITIONS {
			device::unregister(&DeviceID {
				type_: DeviceType::Block,
				major,
				minor: i as _,
			})?;
		}

		Ok(())
	}

	// TODO Handle the case where there is more devices that the number of devices
	// that can be handled in the range of minor numbers
	// TODO When failing, remove previously registered devices
	/// Adds the given storage device to the manager.
	fn add(&mut self, storage: SharedPtr<dyn StorageInterface>) -> Result<(), Errno> {
		// The device files' major number
		let major = self.major_block.get_major();
		// The id of the storage interface in the manager's list
		let storage_id = self.interfaces.len() as u32;

		// The prefix is the path of the main device file
		let mut prefix = String::from(b"/dev/sd")?;
		// TODO Handle if out of the alphabet
		prefix.push(b'a' + (storage_id as u8))?;
		// The path of the main device file
		let main_path = Path::from_str(prefix.as_bytes(), false)?;

		// Creating the main device file
		let main_handle = StorageDeviceHandle::new(
			storage.new_weak(),
			None,
			major,
			storage_id,
			prefix.failable_clone()?
		);
		let main_device = Device::new(
			DeviceID {
				type_: DeviceType::Block,
				major,
				minor: storage_id * MAX_PARTITIONS as u32,
			},
			main_path,
			STORAGE_MODE,
			main_handle,
		)?;
		device::register(main_device)?;

		Self::read_partitions(
			storage.new_weak(),
			major,
			storage_id,
			prefix
		)?;

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
	/// `seed` is the seed for pseudo random generation. The function will set
	/// this variable to another value for the next iteration.
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
	/// If every tests pass, the function returns `true`. Else, it returns
	/// `false`.
	#[cfg(config_debug_storagetest)]
	fn perform_test(&mut self) -> bool {
		let mut seed = 42;
		let iterations_count = 10;
		for i in 0..iterations_count {
			let interfaces_count = self.interfaces.len();

			for j in 0..interfaces_count {
				let interface = &mut self.interfaces[j];

				crate::print!(
					"Processing iteration: {}/{}; device: {}/{}...",
					i + 1,
					iterations_count,
					j + 1,
					interfaces_count
				);

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
	/// The execution of this function removes all the data on every connected
	/// writable disks, so it must be used carefully.
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
	fn get_name(&self) -> &'static str {
		"storage"
	}

	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> Result<(), Errno> {
		// Ignoring non-storage devices
		if dev.get_class() != pci::CLASS_MASS_STORAGE_CONTROLLER {
			return Ok(());
		}

		match dev.get_subclass() {
			// IDE controller
			0x01 => {
				let ide = ide::Controller::new(dev);

				oom::wrap(|| {
					for interface in ide.detect_all()? {
						match self.add(interface) {
							Err(e) if e == errno!(ENOMEM) => return Err(e),
							Err(e) => return Ok(Err(e)),

							_ => {}
						}
					}

					Ok(Ok(()))
				})?;
			}

			// TODO Handle other controller types
			_ => {}
		}

		Ok(())
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) -> Result<(), Errno> {
		// TODO remove device
		todo!();
	}
}
