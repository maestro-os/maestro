//! The device manager is the structure which links the physical devices to device files.

use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;

/// Trait representing a physical device.
pub trait PhysicalDevice {
	/// Returns the product ID of the device.
	fn get_product_id(&self) -> u16;
	/// Returns the vendor ID of the device.
	fn get_vendor_id(&self) -> u16;

	/// Returns the class of the device.
	fn get_class(&self) -> u16;
	/// Returns the subclass of the device.
	fn get_subclass(&self) -> u16;

	/// Tells whether the device is a hotplug device or not.
	fn is_hotplug(&self) -> bool;
}

/// Trait representing a structure managing the link between physical devices and device files.
pub trait DeviceManager {
	/// Detects devices the legacy way.
	/// **WARNING**: This function must be called only once.
	fn legacy_detect(&mut self) -> Result<(), Errno>;

	/// Function called when a new device is plugged in.
	fn on_plug(&mut self, dev: &dyn PhysicalDevice);

	/// Function called when a device is plugged out.
	fn on_unplug(&mut self, dev: &dyn PhysicalDevice);
}

/// The list of device managers.
static mut DEVICE_MANAGERS: Mutex<Vec<Box<dyn DeviceManager>>> = Mutex::new(Vec::new());

/// Registers the given device manager.
pub fn register_manager<M: 'static + DeviceManager>(manager: M) -> Result<(), Errno> {
	let mutex = unsafe {
		&mut DEVICE_MANAGERS
	};
	let mut guard = MutexGuard::new(mutex);
	let device_managers = guard.get_mut();

	let b = Box::new(manager)?;
	device_managers.push(b)
}

/// Function that is called when a new device is plugged in.
/// `dev` is the device that has been plugged in.
pub fn on_plug(dev: &dyn PhysicalDevice) {
	let mutex = unsafe {
		&mut DEVICE_MANAGERS
	};
	let mut guard = MutexGuard::new(mutex);
	let device_managers = guard.get_mut();

	for i in 0..device_managers.len() {
		device_managers[i].on_plug(dev);
	}
}

/// Function that is called when a device is plugged out.
/// `dev` is the device that has been plugged out.
pub fn on_unplug(dev: &dyn PhysicalDevice) {
	let mutex = unsafe {
		&mut DEVICE_MANAGERS
	};
	let mut guard = MutexGuard::new(mutex);
	let device_managers = guard.get_mut();

	for i in 0..device_managers.len() {
		device_managers[i].on_unplug(dev);
	}
}
