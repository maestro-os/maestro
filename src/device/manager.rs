//! The device manager is the structure which links the physical devices to
//! device files.

use crate::device::bar::BAR;
use crate::errno::Errno;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::ptr::arc::Weak;
use core::any::Any;
use core::any::TypeId;

/// Trait representing a physical device.
pub trait PhysicalDevice {
	/// Returns the device ID of the device.
	fn get_device_id(&self) -> u16;
	/// Returns the vendor ID of the device.
	fn get_vendor_id(&self) -> u16;

	/// Returns the command register if present.
	fn get_command_reg(&self) -> Option<u16>;
	/// Returns the status register if present.
	fn get_status_reg(&self) -> Option<u16>;

	/// Returns the class of the device.
	fn get_class(&self) -> u16;
	/// Returns the subclass of the device.
	fn get_subclass(&self) -> u16;
	/// The id of a read-only register that specifies a register-level
	/// programming interface of the device. If not applicable, the function
	/// returns zero.
	fn get_prog_if(&self) -> u8;

	/// Tells whether the device is a hotplug device or not.
	fn is_hotplug(&self) -> bool;

	/// Returns the `n`'th BAR.
	/// If the BAR doesn't exist, the function returns `None`.
	fn get_bar(&self, n: u8) -> Option<BAR>;
}

/// Trait representing a structure managing the link between physical devices
/// and device files.
pub trait DeviceManager: Any {
	/// Function called when a new device is plugged in.
	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> Result<(), Errno>;

	/// Function called when a device is plugged out.
	fn on_unplug(&mut self, dev: &dyn PhysicalDevice) -> Result<(), Errno>;
}

/// The list of device managers.
static DEVICE_MANAGERS: Mutex<HashMap<TypeId, Arc<Mutex<dyn DeviceManager>>>> =
	Mutex::new(HashMap::new());

/// Registers the given device manager.
pub fn register_manager<M: DeviceManager>(manager: M) -> Result<(), Errno> {
	let m = Arc::new(Mutex::new(manager))?;

	let mut device_managers = DEVICE_MANAGERS.lock();
	device_managers.insert(TypeId::of::<M>(), m)?;

	Ok(())
}

/// Returns the device manager with the given type. If the manager is not registered, the function
/// returns `None`.
pub fn get<M: DeviceManager>() -> Option<Weak<Mutex<dyn DeviceManager>>> {
	let device_managers = DEVICE_MANAGERS.lock();
	let dev = device_managers.get(&TypeId::of::<M>())?;

	Some(Arc::downgrade(dev))
}

/// Function that is called when a new device is plugged in.
///
/// `dev` is the device that has been plugged in.
pub fn on_plug(dev: &dyn PhysicalDevice) -> Result<(), Errno> {
	let device_managers = DEVICE_MANAGERS.lock();

	for (_, m) in device_managers.iter() {
		let mut manager = m.lock();
		manager.on_plug(dev)?;
	}

	Ok(())
}

/// Function that is called when a device is plugged out.
///
/// `dev` is the device that has been plugged out.
pub fn on_unplug(dev: &dyn PhysicalDevice) -> Result<(), Errno> {
	let device_managers = DEVICE_MANAGERS.lock();

	for (_, m) in device_managers.iter() {
		let mut manager = m.lock();
		manager.on_unplug(dev)?;
	}

	Ok(())
}
