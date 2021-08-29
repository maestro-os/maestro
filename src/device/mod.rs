//! This module handles device and buses.
//! A device file is an interface with a device of the system, which can be internal or external,
//! or even virtual such as a TTY.

pub mod bus;
pub mod default;
pub mod id;
pub mod keyboard;
pub mod manager;
pub mod ps2;
pub mod serial;
pub mod storage;

use crate::device::manager::DeviceManager;
use crate::errno::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::file::path::Path;
use crate::file;
use crate::util::FailableClone;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::ptr::SharedPtr;
use keyboard::KeyboardManager;
//use storage::StorageManager;

/// Enumeration representing the type of the device.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DeviceType {
	/// A block device.
	Block,
	/// A char device.
	Char,
}

/// Trait providing a interface for device I/O.
pub trait DeviceHandle {
	/// Returns the size of the device in bytes.
	fn get_size(&self) -> u64;

	/// Reads data from the device and writes it to the buffer `buff`.
	/// `offset` is the offset in the file.
	/// The function returns the number of bytes read.
	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<usize, Errno>;
	/// Writes data to the device, reading it from the buffer `buff`.
	/// `offset` is the offset in the file.
	/// The function returns the number of bytes written.
	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<usize, Errno>;
}

/// Structure representing a device, either a block device or a char device. Each device has a
/// major and a minor number.
pub struct Device {
	/// The major number.
	major: u32,
	/// The minor number.
	minor: u32,

	/// The path to the device file.
	path: Path,
	/// The file's mode.
	mode: Mode,
	/// The type of the device.
	type_: DeviceType,

	/// The object handling the device I/O.
	handle: Box<dyn DeviceHandle>,
}

impl Device {
	/// Creates a new instance.
	/// `major` and `minor` are the major and minor numbers of the device.
	/// `type_` is the type of the device.
	/// `handle` is the handle for I/O operations.
	pub fn new<H: 'static + DeviceHandle>(major: u32, minor: u32, path: Path, mode: Mode,
		type_: DeviceType, handle: H) -> Result<Self, Errno> {
		Ok(Self {
			major,
			minor,

			path,
			mode,
			type_,

			handle: Box::new(handle)?,
		})
	}

	/// Returns the major number.
	#[inline]
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Returns the minor number.
	#[inline]
	pub fn get_minor(&self) -> u32 {
		self.minor
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

	/// Returns the type of the device.
	#[inline]
	pub fn get_type(&self) -> DeviceType {
		self.type_
	}

	/// Returns the device number.
	#[inline]
	pub fn get_device_number(&self) -> u64 {
		id::makedev(self.major, self.minor)
	}

	/// Returns the handle of the device for I/O operations.
	#[inline]
	pub fn get_handle(&mut self) -> &mut dyn DeviceHandle {
		self.handle.as_mut()
	}

	// TODO Put file creation on the userspace side?
	/// Creates the device file associated with the structure. If the file already exist, the
	/// function does nothing.
	pub fn create_file(&mut self) -> Result<(), Errno> {
		let file_content = match self.type_ {
			DeviceType::Block => FileContent::BlockDevice(self.major, self.minor),
			DeviceType::Char => FileContent::CharDevice(self.major, self.minor),
		};

		let path_len = self.path.get_elements_count();
		let filename = self.path[path_len - 1].failable_clone()?;

		let mut dir_path = self.path.failable_clone()?;
		dir_path.pop();
		file::create_dirs(&dir_path)?;

		let file = File::new(filename, file_content, 0, 0, self.mode)?;

		let mutex = file::get_files_cache();
		let mut guard = mutex.lock(true);
		let files_cache = guard.get_mut();
		// TODO Cancel directories creation on fail
		files_cache.create_file(&dir_path, file)?;

		Ok(())
	}

	/// If exists, removes the device file. iF the file doesn't exist, the function does nothing.
	pub fn remove_file(&mut self) {
		let mutex = file::get_files_cache();
		let mut guard = mutex.lock(true);
		let files_cache = guard.get_mut();

		if let Ok(mut file) = files_cache.get_file_from_path(&self.path) {
			let mut guard = file.lock(true);
			guard.get_mut().unlink();
		}
	}
}

impl Drop for Device {
	fn drop(&mut self) {
		self.remove_file();
	}
}

/// The list of registered block devices.
static mut BLOCK_DEVICES: Mutex<Vec<SharedPtr<Device>>> = Mutex::new(Vec::new());
/// The list of registered block devices.
static mut CHAR_DEVICES: Mutex<Vec<SharedPtr<Device>>> = Mutex::new(Vec::new());

/// Registers the given device. If the minor/major number is already used, the function fails.
/// The function *doesn't* create the device file.
pub fn register_device(device: Device) -> Result<(), Errno> {
	let mut guard = match device.get_type() {
		DeviceType::Block => {
			unsafe { // Safe because using mutex
				BLOCK_DEVICES.lock(true)
			}
		},
		DeviceType::Char => {
			unsafe { // Safe because using mutex
				CHAR_DEVICES.lock(true)
			}
		}
	};
	let container = guard.get_mut();

	let device_number = device.get_device_number();
	let index = container.binary_search_by(| d | {
		let dn = unsafe {
			d.get_mut().get_mut_payload()
		}.get_device_number();

		device_number.cmp(&dn)
	});
	let index = match index {
		Ok(i) => i,
		Err(i) => i,
	};

	container.insert(index, SharedPtr::new(device)?)
}

// TODO Function to remove a device

/// Returns a mutable reference to the device with the given major number, minor number and type.
/// If the device doesn't exist, the function returns None.
pub fn get_device(type_: DeviceType, major: u32, minor: u32) -> Option<SharedPtr<Device>> {
	let mut guard = match type_ {
		DeviceType::Block => {
			unsafe { // Safe because using mutex
				BLOCK_DEVICES.lock(true)
			}
		},
		DeviceType::Char => {
			unsafe { // Safe because using mutex
				CHAR_DEVICES.lock(true)
			}
		}
	};
	let container = guard.get_mut();

	let device_number = id::makedev(major, minor);
	let index = container.binary_search_by(| d | {
		let dn = unsafe {
			d.get_mut().get_mut_payload()
		}.get_device_number();

		device_number.cmp(&dn)
	});

	if let Ok(i) = index {
		Some(container[i].clone())
	} else {
		None
	}
}

/// Returns the device with the given path `path`.
/// This function is `O(n)` in time.
/// If no device with the given path is found, the function returns None.
pub fn get_by_path(path: &Path) -> Option<SharedPtr<Device>> {
	{
		let mut block_guard = unsafe { // Safe because using Mutex
			BLOCK_DEVICES.lock(true)
		};
		let block_container = block_guard.get_mut();
		for i in 0..block_container.len() {
			let dev_guard = block_container[i].lock(true);
			let dev = dev_guard.get();

			if dev.get_path() == path {
				drop(dev_guard);
				return Some(block_container[i].clone());
			}
		}
	}

	{
		let mut char_guard = unsafe { // Safe because using Mutex
			CHAR_DEVICES.lock(true)
		};
		let char_container = char_guard.get_mut();
		for i in 0..char_container.len() {
			let dev_guard = char_container[i].lock(true);
			let dev = dev_guard.get();

			if dev.get_path() == path {
				drop(dev_guard);
				return Some(char_container[i].clone());
			}
		}
	}

	None
}

/// Initializes devices management.
pub fn init() -> Result<(), Errno> {
	let mut keyboard_manager = KeyboardManager::new();
	keyboard_manager.legacy_detect()?;
	manager::register_manager(keyboard_manager)?;

	// TODO fix
	//let mut storage_manager = StorageManager::new()?;
	//storage_manager.legacy_detect()?;

	//#[cfg(config_debug_storagetest)]
	//storage_manager.test(); // TODO Move after bus detection

	//manager::register_manager(storage_manager)?;

	bus::detect()?;

	Ok(())
}
