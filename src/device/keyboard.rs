//! This module implements the keyboard device manager.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::ps2;
use crate::errno::Errno;
use crate::errno;

/// Structure managing keyboard devices.
/// The manager has the name `kbd`.
pub struct KeyboardManager {
	/// The PS/2 handler.
	ps2_handler: Option<ps2::PS2Handler>,
}

impl KeyboardManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		let s = Self {
			ps2_handler: None,
		};
		s.init_device_files();
		s
	}

	/// Initializes devices files.
	fn init_device_files(&self) {
		// TODO Create /dev/input/event* files
	}

	/// Destroyes devices files.
	fn fini_device_files(&self) {
		// TODO Remove /dev/input/event* files
	}
}

impl DeviceManager for KeyboardManager {
	fn get_name(&self) -> &str {
		"kbd"
	}

	fn legacy_detect(&mut self) -> Result<(), Errno> {
		self.ps2_handler = Some(ps2::PS2Handler::new().or(Err(errno::EIO))?);
		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}

impl Drop for KeyboardManager {
	fn drop(&mut self) {
		self.fini_device_files();
	}
}
