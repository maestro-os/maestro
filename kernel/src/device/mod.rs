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

//! Devices, buses and peripherals implementation.
//!
//! A device file is an interface with a device of the system, which can be
//! internal or external, or even virtual such as a TTY.
//!
//! Since files management requires devices to be initialized in order to access filesystems, the
//! system first needs to initialize devices. However, at that stage, device files cannot be
//! created.
//!
//! Thus, devices are initialized in stages:
//! - **stage 1**: files management is not yet initialized, which means device files are not
//!   created when devices are registered
//! - **stage 2**: files management is initialized, device files can be created. When switching to
//!   that stage, the files of all device that are already registered are created

pub mod bar;
pub mod bus;
pub mod default;
pub mod id;
pub mod keyboard;
pub mod manager;
pub mod serial;
pub mod storage;
pub mod tty;

use crate::{
	device::{
		manager::DeviceManager,
		storage::{PartitionOps, partition::Partition},
	},
	file,
	file::{
		File, FileType, Mode, Stat,
		fs::FileOps,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::{
		buddy::ZONE_KERNEL,
		cache::{MappedNode, RcPage},
		user::UserSlice,
	},
	sync::{mutex::Mutex, spin::Spin},
	syscall::ioctl,
};
use core::{ffi::c_void, fmt, hint::likely, num::NonZeroU64};
use keyboard::KeyboardManager;
use storage::StorageManager;
use utils::{
	boxed::Box,
	collections::{
		hashmap::HashMap,
		path::{Path, PathBuf},
		vec::Vec,
	},
	errno,
	errno::{AllocResult, ENOENT, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

/// Enumeration representing the type of the device.
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeviceType {
	Block,
	Char,
}

impl DeviceType {
	/// Returns the file type associated with the device type.
	pub fn to_file_type(self) -> FileType {
		match self {
			DeviceType::Block => FileType::BlockDevice,
			DeviceType::Char => FileType::CharDevice,
		}
	}
}

/// Creates a device file.
///
/// Arguments:
/// - `id` is the ID of the device
/// - `dev_type` is the device type
/// - `path` is the path of the device file
/// - `perms` is the permissions of the device file
///
/// If the file already exist, the function does nothing.
pub fn create_file(id: &DeviceID, dev_type: DeviceType, path: &Path, perms: Mode) -> EResult<()> {
	// Create the parent directory in which the device file is located
	let parent_path = path.parent().unwrap_or(Path::root());
	file::util::create_dirs(parent_path)?;
	// Resolve path
	let resolved = vfs::resolve_path(path, &ResolutionSettings::cur_task(true, true))?;
	match resolved {
		Resolved::Creatable {
			parent,
			name,
		} => {
			// Create the device file
			vfs::create_file(
				parent,
				name,
				Stat {
					mode: dev_type.to_file_type().to_mode() | perms,
					dev_major: id.major,
					dev_minor: id.minor,
					..Default::default()
				},
			)?;
			Ok(())
		}
		// The file exists, do nothing
		Resolved::Found(_) => Ok(()),
	}
}

/// If it exists, removes the file at `path`.
pub fn remove_file(path: &Path) -> EResult<()> {
	let res = vfs::get_file_from_path(path, true);
	let ent = match res {
		Ok(ent) => ent,
		Err(e) if e.as_int() == ENOENT => return Ok(()),
		Err(e) => return Err(e),
	};
	vfs::unlink(ent)
}

/// A device type, major and minor, who act as a unique ID for a device.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DeviceID {
	/// The major number.
	pub major: u32,
	/// The minor number.
	pub minor: u32,
}

impl DeviceID {
	/// Returns the device number.
	pub fn get_device_number(&self) -> u64 {
		id::makedev(self.major, self.minor)
	}
}

/// Device I/O interface.
///
/// This trait makes use of **interior mutability** to allow concurrent accesses.
pub trait BlockDeviceOps: fmt::Debug {
	/// Returns the device ID and path for a device representing the partition `id`.
	fn new_partition(&self, dev: &BlkDev, id: u32) -> AllocResult<(DeviceID, PathBuf)>;

	/// Drops the device ID used by a partition.
	fn drop_partition(&self, dev: &BlkDev) {
		let _ = dev;
	}

	/// Reads a page of data from the device.
	///
	/// `off` is the offset of the page, in pages
	fn read_page(&self, dev: &Arc<BlkDev>, off: u64) -> EResult<RcPage>;

	/// Writes a page of data back to the device.
	///
	/// `off` is the offset of the page, in pages
	fn writeback(&self, dev: &BlkDev, off: u64, page: &RcPage) -> EResult<()>;

	/// Polls the device with the given mask.
	fn poll(&self, dev: &BlkDev, mask: u32) -> EResult<u32> {
		let _ = (dev, mask);
		Err(errno!(EINVAL))
	}

	/// Performs an ioctl operation on the device.
	///
	/// Arguments:
	/// - `request` is the ID of the request to perform
	/// - `argp` is a pointer to the argument
	fn ioctl(&self, dev: &BlkDev, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let _ = (dev, request, argp);
		Err(errno!(EINVAL))
	}
}

/// A block device.
#[derive(Debug)]
pub struct BlkDev {
	/// The device's ID
	pub id: DeviceID,
	/// The path to the device file
	pub path: PathBuf,
	/// The file's mode
	pub mode: Mode,

	/// Size of a block in bytes
	pub blk_size: NonZeroU64,
	/// Number of blocks on the device
	pub blk_count: u64,

	/// Tells whether this is a partition device
	pub is_partition: bool,
	/// The list of associated partition devices
	pub(crate) partitions: Mutex<Vec<Arc<BlkDev>>, false>,

	/// The device I/O interface
	pub ops: Box<dyn BlockDeviceOps>,
	/// The device as a mapped node
	pub(crate) mapped: MappedNode,
}

impl BlkDev {
	fn new_impl(dev: BlkDev) -> EResult<Arc<Self>> {
		let dev = Arc::new(dev)?;
		if likely(file::is_init()) {
			create_file(&dev.id, DeviceType::Block, &dev.path, dev.mode)?;
		}
		Ok(dev)
	}

	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the device's ID
	/// - `path` is the path to the device's file
	/// - `mode` is the set of permissions associated with the device's file
	/// - `blk_size` is the block size on the device
	/// - `blk_count` is the block count on the device
	/// - `ops` is the handle for I/O operations
	pub fn new(
		id: DeviceID,
		path: PathBuf,
		mode: Mode,
		blk_size: NonZeroU64,
		blk_count: u64,
		ops: Box<dyn BlockDeviceOps>,
	) -> EResult<Arc<Self>> {
		Self::new_impl(Self {
			id,
			path,
			mode,

			blk_size,
			blk_count,

			is_partition: false,
			partitions: Mutex::new(Vec::new()),

			ops,
			mapped: Default::default(),
		})
	}

	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the device's ID
	/// - `path` is the path to the device's file
	/// - `mode` is the set of permissions associated with the device's file
	/// - `dev` is the device containing the partition
	/// - `partition` is the partition
	pub fn new_partition(
		id: DeviceID,
		path: PathBuf,
		mode: Mode,
		dev: Arc<BlkDev>,
		partition: Partition,
	) -> EResult<Arc<Self>> {
		Self::new_impl(Self {
			id,
			path,
			mode,

			blk_size: dev.blk_size,
			blk_count: partition.size,

			is_partition: true,
			partitions: Mutex::new(Vec::new()),

			ops: Box::new(PartitionOps {
				dev,
				partition,
			})?,
			mapped: Default::default(),
		})
	}

	/// Allocates a blank page for I/O.
	///
	/// This function is meant to be used in [`BlockDeviceOps::read_page`].
	pub fn new_page(this: &Arc<Self>, off: u64) -> AllocResult<RcPage> {
		RcPage::new(ZONE_KERNEL, Some(this.clone()), off)
	}

	/// Removes the device file from the filesystem
	#[inline]
	pub fn remove_file(&self) -> EResult<()> {
		remove_file(&self.path)
	}
}

impl Drop for BlkDev {
	fn drop(&mut self) {
		let _ = self.remove_file();
	}
}

/// A character device.
#[derive(Debug)]
pub struct CharDev {
	/// The device's ID
	pub id: DeviceID,
	/// The path to the device file
	pub path: PathBuf,
	/// The file's mode
	pub mode: Mode,

	/// The device I/O interface
	pub ops: Box<dyn FileOps>,
}

impl CharDev {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the device's ID
	/// - `path` is the path to the device's file
	/// - `mode` is the set of permissions associated with the device's file
	/// - `handle` is the handle for I/O operations
	pub fn new<IO: 'static + FileOps>(
		id: DeviceID,
		path: PathBuf,
		mode: Mode,
		ops: IO,
	) -> EResult<Arc<Self>> {
		let dev = Arc::new(Self {
			id,

			path,
			mode,

			ops: Box::new(ops)?,
		})?;
		if likely(file::is_init()) {
			create_file(&id, DeviceType::Char, &dev.path, mode)?;
		}
		Ok(dev)
	}
}

impl Drop for CharDev {
	fn drop(&mut self) {
		let _ = remove_file(&self.path);
	}
}

/// The list of registered block devices.
pub static BLK_DEVICES: Spin<HashMap<DeviceID, Arc<BlkDev>>> = Spin::new(HashMap::new());
/// The list of registered character devices.
pub static CHAR_DEVICES: Spin<HashMap<DeviceID, Arc<CharDev>>> = Spin::new(HashMap::new());

/// Helper to insert a block device.
#[inline]
pub fn register_blk(dev: Arc<BlkDev>) -> AllocResult<()> {
	BLK_DEVICES.lock().insert(dev.id, dev)?;
	Ok(())
}

/// Helper to insert a character device.
#[inline]
pub fn register_char(dev: Arc<CharDev>) -> AllocResult<()> {
	CHAR_DEVICES.lock().insert(dev.id, dev)?;
	Ok(())
}

/// Block device file operations.
#[derive(Debug)]
pub struct BlkDevFileOps;

impl FileOps for BlkDevFileOps {
	fn read(&self, file: &File, mut off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let dev = file.as_block_device().ok_or_else(|| errno!(ENODEV))?;
		let start = off / PAGE_SIZE as u64;
		let end = off
			.checked_add(buf.len() as u64)
			.ok_or_else(|| errno!(EOVERFLOW))?
			.div_ceil(PAGE_SIZE as u64);
		let mut buf_off = 0;
		for page_off in start..end {
			let page = dev.ops.read_page(&dev, page_off)?;
			let inner_off = off as usize % PAGE_SIZE;
			let len = unsafe {
				let page_ptr = page.virt_addr().as_ptr::<u8>().add(inner_off);
				buf.copy_to_user_raw(buf_off, page_ptr, PAGE_SIZE - inner_off)?
			};
			buf_off += len;
			off += len as u64;
		}
		Ok(buf_off)
	}

	fn write(&self, file: &File, mut off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let dev = file.as_block_device().ok_or_else(|| errno!(ENODEV))?;
		let start = off / PAGE_SIZE as u64;
		let end = off
			.checked_add(buf.len() as u64)
			.ok_or_else(|| errno!(EOVERFLOW))?
			.div_ceil(PAGE_SIZE as u64);
		let mut buf_off = 0;
		for page_off in start..end {
			let page = dev.ops.read_page(&dev, page_off)?;
			let inner_off = off as usize % PAGE_SIZE;
			let len = unsafe {
				let page_ptr = page.virt_addr().as_ptr::<u8>().add(inner_off);
				buf.copy_from_user_raw(buf_off, page_ptr, PAGE_SIZE - inner_off)?
			};
			page.mark_dirty();
			buf_off += len;
			off += len as u64;
		}
		Ok(buf_off)
	}

	fn poll(&self, file: &File, mask: u32) -> EResult<u32> {
		let dev = file.as_block_device().ok_or_else(|| errno!(ENODEV))?;
		dev.ops.poll(&dev, mask)
	}

	fn ioctl(&self, file: &File, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let dev = file.as_block_device().ok_or_else(|| errno!(ENODEV))?;
		dev.ops.ioctl(&dev, request, argp)
	}
}

/// Initializes devices management.
pub(crate) fn init() -> EResult<()> {
	id::init()?;
	let keyboard_manager = KeyboardManager::new();
	manager::register(keyboard_manager)?;
	let storage_manager = StorageManager::new()?;
	manager::register(storage_manager)?;
	bus::detect()?;
	// Testing disk I/O (if enabled)
	#[cfg(config_debug_storage_test)]
	{
		let storage_manager_mutex = manager::get::<StorageManager>().unwrap();
		let mut storage_manager = storage_manager_mutex.lock();
		(&mut *storage_manager as &mut dyn core::any::Any)
			.downcast_mut::<StorageManager>()
			.unwrap()
			.test();
	}

	Ok(())
}

/// Switches to stage 2, creating device files of devices that are already registered.
///
/// This function must be used only once at boot, after files management has been initialized.
pub(crate) fn stage2() -> EResult<()> {
	default::create().unwrap_or_else(|e| panic!("Failed to create default devices! ({e})"));
	// Create device files
	let devs = BLK_DEVICES.lock();
	for (id, dev) in devs.iter() {
		create_file(id, DeviceType::Block, &dev.path, dev.mode)?;
	}
	let devs = CHAR_DEVICES.lock();
	for (id, dev) in devs.iter() {
		create_file(id, DeviceType::Char, &dev.path, dev.mode)?;
	}
	Ok(())
}
