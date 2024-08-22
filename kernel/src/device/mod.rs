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
pub mod driver;
pub mod id;
pub mod keyboard;
pub mod manager;
pub mod serial;
pub mod storage;
pub mod tty;

use crate::{
	device::manager::DeviceManager,
	file,
	file::{
		path::{Path, PathBuf},
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType, Mode, Stat,
	},
	syscall::ioctl,
};
use core::{ffi::c_void, fmt, num::NonZeroU64};
use keyboard::KeyboardManager;
use storage::StorageManager;
use utils::{
	collections::{hashmap::HashMap, vec::Vec},
	errno,
	errno::{AllocResult, CollectResult, EResult},
	lock::Mutex,
	ptr::arc::Arc,
	slice_copy, vec, TryClone,
};

/// Enumeration representing the type of the device.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeviceType {
	/// A block device.
	Block,
	/// A char device.
	Char,
}

impl DeviceType {
	/// Returns the file type associated with the device type.
	pub fn as_file_type(&self) -> FileType {
		match self {
			DeviceType::Block => FileType::BlockDevice,
			DeviceType::Char => FileType::CharDevice,
		}
	}
}

impl fmt::Display for DeviceType {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		fmt::Debug::fmt(self, fmt)
	}
}

/// A device type, major and minor, who act as a unique ID for a device.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeviceID {
	/// The type of the device.
	pub dev_type: DeviceType,
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
pub trait DeviceIO {
	/// Returns the granularity of I/O for the device, in bytes.
	fn block_size(&self) -> NonZeroU64;
	/// Returns the number of blocks on the device.
	fn blocks_count(&self) -> u64;

	/// Reads data from the device.
	///
	/// Arguments:
	/// - `off` is the offset on the device, in blocks
	/// - `buf` is the buffer to which the data is written
	///
	/// The size of the buffer has to be a multiple of the block size.
	///
	/// On success, the function returns the number of bytes read.
	fn read(&self, off: u64, buf: &mut [u8]) -> EResult<usize>;

	/// Writes data to the device.
	///
	/// Arguments:
	/// - `off` is the offset on the device, in blocks
	/// - `buf` is the buffer from which the data is read
	///
	/// The size of the buffer has to be a multiple of the block size.
	///
	/// On success, the function returns the number of bytes written.
	fn write(&self, off: u64, buf: &[u8]) -> EResult<usize>;

	/// Reads data from the device.
	///
	/// Contrary to [`Self::read`], `off` is in bytes and no block alignment is required.
	fn read_bytes(&self, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let blk_size = self.block_size().get();
		let mut blk = vec![0u8; blk_size as usize]?;
		let start = off / blk_size;
		let end = off
			.checked_add(buf.len() as u64)
			.ok_or_else(|| errno!(EOVERFLOW))?
			/ blk_size;
		let mut buf_off = 0;
		for i in start..end {
			self.read(i, &mut blk)?;
			let inner_off = (off % blk_size) as usize;
			buf_off += slice_copy(&blk[inner_off..], &mut buf[buf_off..]);
		}
		Ok(buf.len())
	}

	/// Writes data to the device.
	///
	/// Contrary to [`Self::write`], `off` is in bytes and no block alignment is required.
	fn write_bytes(&self, off: u64, buf: &[u8]) -> EResult<usize> {
		let blk_size = self.block_size().get();
		let mut blk = vec![0u8; blk_size as usize]?;
		let start = off / blk_size;
		let end = off
			.checked_add(buf.len() as u64)
			.ok_or_else(|| errno!(EOVERFLOW))?
			/ blk_size;
		let mut buf_off = 0;
		for i in start..end {
			self.read(i, &mut blk)?;
			let inner_off = (off % blk_size) as usize;
			buf_off += slice_copy(&buf[buf_off..], &mut blk[inner_off..]);
			self.write(i, &blk)?;
		}
		Ok(buf.len())
	}

	/// Performs an ioctl operation on the device.
	///
	/// Arguments:
	/// - `request` is the ID of the request to perform
	/// - `argp` is a pointer to the argument
	fn ioctl(&self, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let _ = (request, argp);
		Err(errno!(EINVAL))
	}
}

/// A device, either a block device or a char device.
///
/// Each device has a major and a minor number.
pub struct Device {
	/// The device's ID.
	id: DeviceID,
	/// The path to the device file.
	path: PathBuf,
	/// The file's mode.
	mode: Mode,

	/// The device I/O interface.
	io: Arc<dyn DeviceIO>,
}

impl Device {
	// TODO accept both `&'static Path` and `PathBuf`?
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `id` is the device's ID.
	/// - `path` is the path to the device's file.
	/// - `mode` is the set of permissions associated with the device's file.
	/// - `handle` is the handle for I/O operations.
	pub fn new<IO: 'static + DeviceIO>(
		id: DeviceID,
		path: PathBuf,
		mode: Mode,
		handle: IO,
	) -> EResult<Self> {
		Ok(Self {
			id,

			path,
			mode,

			io: Arc::new(handle)?,
		})
	}

	/// Returns the device ID.
	#[inline]
	pub fn get_id(&self) -> &DeviceID {
		&self.id
	}

	/// Returns the path to the device file.
	#[inline]
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns the device file's mode.
	#[inline]
	pub fn get_mode(&self) -> Mode {
		self.mode
	}

	/// Returns the I/O interface.
	#[inline]
	pub fn get_io(&self) -> &Arc<dyn DeviceIO> {
		&self.io
	}

	/// Creates a device file.
	///
	/// Arguments:
	/// - `id` is the ID of the device.
	/// - `path` is the path of the device file.
	/// - `perms` is the permissions of the device file.
	///
	/// If the file already exist, the function does nothing.
	///
	/// The function takes a mutex guard because it needs to unlock the device
	/// in order to create the file without a deadlock since the VFS accesses a device to write on
	/// the filesystem.
	pub fn create_file(id: &DeviceID, path: &Path, perms: Mode) -> EResult<()> {
		// Create the parent directory in which the device file is located
		let parent_path = path.parent().unwrap_or(Path::root());
		file::util::create_dirs(parent_path)?;
		// Resolve path
		let resolved = vfs::resolve_path(
			path,
			&ResolutionSettings {
				create: true,
				..ResolutionSettings::kernel_follow()
			},
		)?;
		match resolved {
			Resolved::Creatable {
				parent,
				name,
			} => {
				// Create the device file
				vfs::create_file(
					parent,
					name,
					&AccessProfile::KERNEL,
					Stat {
						mode: id.dev_type.as_file_type().to_mode() | perms,
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

	/// If exists, removes the device file.
	///
	/// If the file doesn't exist, the function does nothing.
	pub fn remove_file(&self) -> EResult<()> {
		vfs::unlink_from_path(&self.path, &ResolutionSettings::kernel_follow())
	}
}

impl Drop for Device {
	fn drop(&mut self) {
		if let Err(_e) = self.remove_file() {
			// TODO Log the error
		}
	}
}

/// The list of registered devices.
static DEVICES: Mutex<HashMap<DeviceID, Arc<Device>>> = Mutex::new(HashMap::new());

/// Registers the given device.
///
/// If the device ID is already used, the function fails.
///
/// If files management is initialized, the function creates the associated device file.
pub fn register(device: Device) -> EResult<()> {
	let id = device.id.clone();
	let path = device.get_path().to_path_buf()?;
	let mode = device.get_mode();
	// Insert
	{
		let dev = Arc::new(device)?;
		let mut devs = DEVICES.lock();
		devs.insert(id.clone(), dev)?;
	}
	// Create file if files management has been initialized
	if file::is_init() {
		Device::create_file(&id, &path, mode)?;
	}
	Ok(())
}

/// Unregisters the device with the given ID.
///
/// If the device doesn't exist, the function does nothing.
///
/// If files management is initialized, the function removes the associated device file.
pub fn unregister(id: &DeviceID) -> EResult<()> {
	let dev = {
		let mut devs = DEVICES.lock();
		devs.remove(id)
	};
	if let Some(dev) = dev {
		dev.remove_file()?;
	}
	Ok(())
}

/// Returns a mutable reference to the device with the given ID.
///
/// If the device doesn't exist, the function returns `None`.
pub fn get(id: &DeviceID) -> Option<Arc<Device>> {
	let devs = DEVICES.lock();
	devs.get(id).cloned()
}

/// Initializes devices management.
pub(crate) fn init() -> EResult<()> {
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
	// Collecting all data to create device files is necessary to avoid a deadlock, because disk
	// accesses require locking the filesystem's device
	let devs_info = {
		let devs = DEVICES.lock();
		devs.iter()
			.map(|(id, dev)| Ok((id.clone(), dev.path.try_clone()?, dev.mode)))
			.collect::<AllocResult<CollectResult<Vec<_>>>>()?
			.0?
	};
	for (id, path, mode) in devs_info {
		Device::create_file(&id, &path, mode)?;
	}
	Ok(())
}
