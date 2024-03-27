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
//! created when devices are registered
//! - **stage 2**: files management is initialized, device files can be created. When switching to
//! that stage, the files of all device that are already registered are created

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
		Mode,
	},
	process::{mem_space::MemSpace, Process},
	syscall::ioctl,
};
use core::{ffi::c_void, fmt};
use keyboard::KeyboardManager;
use storage::StorageManager;
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, vec::Vec},
	errno::{AllocResult, CollectResult, EResult},
	io::IO,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	TryClone,
};

/// Enumeration representing the type of the device.
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeviceType {
	/// A block device.
	Block,
	/// A char device.
	Char,
}

impl fmt::Display for DeviceType {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		match self {
			Self::Block => write!(fmt, "Block"),
			Self::Char => write!(fmt, "Char"),
		}
	}
}

/// A structure grouping a device type, a device major and a device minor, which acts as a unique
/// ID.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct DeviceID {
	/// The type of the device.
	pub type_: DeviceType,
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

/// Trait providing a interface for device I/O.
pub trait DeviceHandle: IO {
	/// Performs an ioctl operation on the device.
	///
	/// Arguments:
	/// - `mem_space` is the memory space on which pointers are to be
	/// dereferenced.
	/// - `request` is the ID of the request to perform.
	/// - `argp` is a pointer to the argument.
	fn ioctl(
		&mut self,
		mem_space: Arc<IntMutex<MemSpace>>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> EResult<u32>;

	/// Adds the given process to the list of processes waiting on the device.
	///
	/// The function sets the state of the process to `Sleeping`.
	/// When the event occurs, the process will be woken up.
	///
	/// `mask` is the mask of poll event to wait for.
	///
	/// If the device cannot block, the function does nothing.
	fn add_waiting_process(&mut self, _proc: &mut Process, _mask: u32) -> EResult<()> {
		Ok(())
	}
}

/// Structure representing a device, either a block device or a char device.
///
/// Each device has a major and a minor number.
pub struct Device {
	/// The device's ID.
	id: DeviceID,

	/// The path to the device file.
	path: PathBuf,
	/// The file's mode.
	mode: Mode,

	/// The object handling the device I/O.
	handle: Box<dyn DeviceHandle>,
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
	pub fn new<H: 'static + DeviceHandle>(
		id: DeviceID,
		path: PathBuf,
		mode: Mode,
		handle: H,
	) -> EResult<Self> {
		Ok(Self {
			id,

			path,
			mode,

			handle: Box::new(handle)?,
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

	/// Returns the handle of the device for I/O operations.
	#[inline]
	pub fn get_handle(&mut self) -> &mut dyn DeviceHandle {
		self.handle.as_mut()
	}

	/// Creates a device file.
	///
	/// Arguments:
	/// - `id` is the ID of the device.
	/// - `path` is the path of the device file.
	/// - `mode` is the permissions of the device file.
	///
	/// If the file already exist, the function does nothing.
	///
	/// The function takes a mutex guard because it needs to unlock the device
	/// in order to create the file without a deadlock since the VFS accesses a device to write on
	/// the filesystem.
	pub fn create_file(id: &DeviceID, path: &Path, mode: Mode) -> EResult<()> {
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
				let mut parent = parent.lock();
				let name = name.try_into()?;
				// Create the device file
				vfs::create_file(
					&mut parent,
					name,
					&AccessProfile::KERNEL,
					mode,
					id.to_file_content(),
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
	pub fn remove_file(&mut self) -> EResult<()> {
		vfs::remove_file_from_path(&self.path, &ResolutionSettings::kernel_follow())
	}
}

impl IO for Device {
	fn get_size(&self) -> u64 {
		self.handle.get_size()
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		self.handle.read(offset, buff)
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> EResult<u64> {
		self.handle.write(offset, buff)
	}

	fn poll(&mut self, mask: u32) -> EResult<u32> {
		self.handle.poll(mask)
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
static DEVICES: Mutex<HashMap<DeviceID, Arc<Mutex<Device>>>> = Mutex::new(HashMap::new());

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
		let dev_mutex = Arc::new(Mutex::new(device))?;
		let mut devs = DEVICES.lock();
		devs.insert(id.clone(), dev_mutex.clone())?;
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
	let dev_mutex = {
		let mut devs = DEVICES.lock();
		devs.remove(id)
	};

	if let Some(dev_mutex) = dev_mutex {
		// Remove file
		let mut dev = dev_mutex.lock();
		dev.remove_file()?;
	}

	Ok(())
}

/// Returns a mutable reference to the device with the given ID.
///
/// If the device doesn't exist, the function returns `None`.
pub fn get(id: &DeviceID) -> Option<Arc<Mutex<Device>>> {
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
			.map(|(id, dev)| {
				let dev = dev.lock();
				Ok((id.clone(), dev.path.try_clone()?, dev.mode))
			})
			.collect::<AllocResult<CollectResult<Vec<_>>>>()?
			.0?
	};
	for (id, path, mode) in devs_info {
		Device::create_file(&id, &path, mode)?;
	}

	Ok(())
}
