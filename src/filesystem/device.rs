/// This module handles device files such as Block Devices and Char devices.
/// A device file is an interface with a device of the system, which can be internal or external,
/// or even virtual such as a TTY.

use crate::errno::Errno;
use crate::util::boxed::Box;

// TODO Unit tests
/// Returns the major number from a device number.
pub fn major(dev: u64) -> u32 {
	(((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff)) as _
}

// TODO Unit tests
/// Returns the minor number from a device number.
pub fn minor(dev: u64) -> u32 {
	((dev & 0xff) | ((dev >> 12) & !0xff)) as _
}

// TODO Unit tests
/// Returns a device number from a major/minor pair.
pub fn makedev(major: u32, minor: u32) -> u64 {
	(((minor & 0xff) as u64)
		| (((major & 0xfff) as u64) << 8)
		| (((minor & !0xff) as u64) << 12)
		| (((major & !0xfff) as u64) << 32)) as _
}

/// Enumeration representing the type of the device.
pub enum DeviceType {
	/// A block device.
	Block,
	/// A char device.
	Char,
}

/// Trait providing a interface for device I/O.
pub trait DeviceHandle {
	/// Reads data from the device and writes it to the buffer `buff`.
	fn read(&mut self, buff: &mut [u8]) -> Result<(), Errno>;
	/// Writes data to the device, reading it from the buffer `buff`.
	fn write(&mut self, buff: &[u8]) -> Result<(), Errno>;
}

/// Structure representing a device, either a block device or a char device. Each device has a
/// major and a minor number.
pub struct Device {
	/// The major number.
	major: u32,
	/// The minor number.
	minor: u32,

	/// The type of the device.
	type_: DeviceType,

	/// The object handling the device I/O.
	handle: Box::<dyn DeviceHandle>,
}

impl Device {
	/// Creates a new instance.
	/// `major` and `minor` are the major and minor numbers of the device.
	/// `type_` is the type of the device.
	/// `handle` is the handle for I/O operations.
	pub fn new<H: 'static + DeviceHandle>(major: u32, minor: u32, type_: DeviceType, handle: H)
		-> Result<Self, Errno> {
		Ok(Self {
			major: major,
			minor: minor,

			type_: type_,

			handle: Box::new(handle)?,
		})
	}

	/// Returns the major number.
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Returns the minor number.
	pub fn get_minor(&self) -> u32 {
		self.minor
	}

	/// Returns the minor number.
	pub fn get_device(&self) -> u64 {
		makedev(self.major, self.minor)
	}

	/// Returns the handle of the device for I/O operations.
	pub fn get_handle(&mut self) -> &mut dyn DeviceHandle {
		self.handle.as_mut()
	}
}

/// Registers the given device.
pub fn register_device(_device: Device) -> Result<(), Errno> {
	// TODO
	Ok(())
}

// TODO Function to get a device
