/// This module implements the keyboard device manager.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::errno::Errno;
use crate::device::ps2;

/// Structure managing keyboard devices.
pub struct KeyboardManager {}

impl KeyboardManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {}
	}
}

impl DeviceManager for KeyboardManager {
	fn legacy_detect(&mut self) -> Result<(), Errno> {
		let _ps2_module = ps2::PS2Module::new(| c, action | {
			crate::println!("Key action! {:?} {:?}", c, action);
			// TODO Write to device file
		});
		// TODO Insert somewhere

		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}
