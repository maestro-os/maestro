//! A driver is a piece of software allowing to use a specific piece of hardware. Such a component
//! is often located inside of a kernel module.

use crate::device::manager::PhysicalDevice;
use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;
use crate::util::ptr::WeakPtr;

/// Trait representing a device driver.
pub trait Driver {
	/// Returns the name of the driver.
	fn get_name(&self) -> &str;

	/// Function called when a new device is plugged in. If the driver is not compatible with the
	/// device, the function shall ignore it.
	fn on_plug(&self, dev: &dyn PhysicalDevice);

	/// Function called when a device in unplugged.
	fn on_unplug(&self, dev: &dyn PhysicalDevice);
}

/// The list of drivers.
static DRIVERS: Mutex<Vec<SharedPtr<dyn Driver>>> = Mutex::new(Vec::new());

/// Registers the given driver.
pub fn register<D: 'static + Driver>(driver: D) -> Result<(), Errno> {
	let mut guard = DRIVERS.lock();
	let drivers = guard.get_mut();

	let m = SharedPtr::new(driver)?;
	drivers.push(m)
}

/// Returns the driver with name `name`.
pub fn get_by_name(name: &str) -> Option<WeakPtr<dyn Driver>> {
	let mut guard = DRIVERS.lock();
	let drivers = guard.get_mut();

	for i in 0..drivers.len() {
		let guard = drivers[i].lock();

		if guard.get().get_name() == name {
			drop(guard);
			return Some(drivers[i].new_weak());
		}
	}

	None
}

/// Function that is called when a new device is plugged in.
/// `dev` is the device that has been plugged in.
pub fn on_plug(dev: &dyn PhysicalDevice) {
	let mut guard = DRIVERS.lock();
	let drivers = guard.get_mut();

	for i in 0..drivers.len() {
		let mut guard = drivers[i].lock();
		let manager = guard.get_mut();
		manager.on_plug(dev);
	}
}

/// Function that is called when a device is plugged out.
/// `dev` is the device that has been plugged out.
pub fn on_unplug(dev: &dyn PhysicalDevice) {
	let mut guard = DRIVERS.lock();
	let drivers = guard.get_mut();

	for i in 0..drivers.len() {
		let mut guard = drivers[i].lock();
		let manager = guard.get_mut();
		manager.on_unplug(dev);
	}
}
