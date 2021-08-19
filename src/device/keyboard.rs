//! This module implements the keyboard device manager.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::errno::Errno;
use crate::device::ps2;

/// Structure managing keyboard devices.
pub struct KeyboardManager {
	/// The PS/2 handler.
	ps2_handler: Option<ps2::PS2Handler>,
}

impl KeyboardManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			ps2_handler: None,
		}
	}
}

impl DeviceManager for KeyboardManager {
	fn legacy_detect(&mut self) -> Result<(), Errno> {
		let mut ps2_handler = ps2::PS2Handler::new(| c, action | {
			crate::println!("Key action! {:?} {:?}", c, action);
			// TODO Write to device file and current TTY
		});
		if ps2_handler.init().is_err() {
			return Err(crate::errno::EIO); // TODO
		}
		self.ps2_handler = Some(ps2_handler);

		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}
