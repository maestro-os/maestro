//! A driver is a piece of software allowing to use a specific piece of hardware. Such a component is often located inside of a kernel module.

use crate::device::manager::PhysicalDevice;

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
