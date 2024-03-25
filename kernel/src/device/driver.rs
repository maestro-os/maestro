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

//! A driver is a piece of software allowing to use a specific piece of
//! hardware. Such a component is often located inside of a kernel module.

use crate::device::manager::PhysicalDevice;
use utils::{
	collections::vec::Vec,
	errno::AllocResult,
	lock::Mutex,
	ptr::arc::{Arc, Weak},
};

/// Trait representing a device driver.
pub trait Driver {
	/// Returns the name of the driver.
	fn get_name(&self) -> &str;

	/// Function called when a new device is plugged in.
	///
	/// If the driver is not compatible with the device, the function shall ignore it.
	fn on_plug(&self, dev: &dyn PhysicalDevice);

	/// Function called when a device in unplugged.
	fn on_unplug(&self, dev: &dyn PhysicalDevice);
}

/// The list of drivers.
static DRIVERS: Mutex<Vec<Arc<Mutex<dyn Driver>>>> = Mutex::new(Vec::new());

/// Registers the given driver.
pub fn register<D: 'static + Driver>(driver: D) -> AllocResult<()> {
	let m = Arc::new(Mutex::new(driver))?;
	let mut drivers = DRIVERS.lock();
	drivers.push(m)
}

/// Unregisters the driver with the given name.
pub fn unregister(_name: &str) {
	// TODO
	todo!();
}

/// Returns the driver with name `name`.
pub fn get_by_name(name: &str) -> Option<Weak<Mutex<dyn Driver>>> {
	let drivers = DRIVERS.lock();
	for driver_mutex in drivers.iter() {
		let driver = driver_mutex.lock();
		if driver.get_name() == name {
			drop(driver);
			return Some(Arc::downgrade(driver_mutex));
		}
	}
	None
}

/// Function that is called when a new device is plugged in.
///
/// `dev` is the device that has been plugged in.
pub fn on_plug(dev: &dyn PhysicalDevice) {
	let drivers = DRIVERS.lock();
	for driver_mutex in drivers.iter() {
		let driver = driver_mutex.lock();
		driver.on_plug(dev);
	}
}

/// Function that is called when a device is plugged out.
///
/// `dev` is the device that has been plugged out.
pub fn on_unplug(dev: &dyn PhysicalDevice) {
	let drivers = DRIVERS.lock();
	for driver_mutex in drivers.iter() {
		let driver = driver_mutex.lock();
		driver.on_unplug(dev);
	}
}
