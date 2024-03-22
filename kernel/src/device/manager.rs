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

//! The device manager is the structure which links the physical devices to
//! device files.

use crate::device::bar::BAR;
use core::any::{Any, TypeId};
use utils::{collections::hashmap::HashMap, errno::EResult, lock::Mutex, ptr::arc::Arc};

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
	/// programming interface of the device.
	///
	/// If not applicable, the function returns zero.
	fn get_prog_if(&self) -> u8;

	/// Tells whether the device is a hotplug device or not.
	fn is_hotplug(&self) -> bool;

	/// Returns the list of available BARs for the device.
	fn get_bars(&self) -> &[Option<BAR>];

	/// Returns the interrupt line used by the device.
	///
	/// If the device doesn't use any, the function returns `None`.
	fn get_interrupt_line(&self) -> Option<u8>;
	/// Returns the interrupt PIN used by the device.
	///
	/// If the device doesn't use any, the function returns `None`.
	fn get_interrupt_pin(&self) -> Option<u8>;
}

/// Trait representing a structure managing the link between physical devices
/// and device files.
pub trait DeviceManager: Any {
	/// Function called when a new device is plugged in.
	fn on_plug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()>;

	/// Function called when a device is plugged out.
	fn on_unplug(&mut self, dev: &dyn PhysicalDevice) -> EResult<()>;
}

/// The list of device managers.
static DEVICE_MANAGERS: Mutex<HashMap<TypeId, Arc<Mutex<dyn DeviceManager>>>> =
	Mutex::new(HashMap::new());

/// Registers the given device manager.
pub fn register<M: DeviceManager>(manager: M) -> EResult<()> {
	let m = Arc::new(Mutex::new(manager))?;
	let mut device_managers = DEVICE_MANAGERS.lock();
	device_managers.insert(TypeId::of::<M>(), m)?;
	Ok(())
}

/// Returns the device manager with the given type. If the manager is not registered, the function
/// returns `None`.
pub fn get<M: DeviceManager>() -> Option<Arc<Mutex<dyn DeviceManager>>> {
	let device_managers = DEVICE_MANAGERS.lock();
	device_managers.get(&TypeId::of::<M>()).cloned()
}

/// Function that is called when a new device is plugged in.
///
/// `dev` is the device that has been plugged in.
pub fn on_plug(dev: &dyn PhysicalDevice) -> EResult<()> {
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
pub fn on_unplug(dev: &dyn PhysicalDevice) -> EResult<()> {
	let device_managers = DEVICE_MANAGERS.lock();
	for (_, m) in device_managers.iter() {
		let mut manager = m.lock();
		manager.on_unplug(dev)?;
	}
	Ok(())
}
