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

//! This module implements the driver structure.

use crate::nic::NIC;
use core::{any::Any, convert::TryInto};
use kernel::{
	device::{bus::pci::PCIManager, manager, manager::PhysicalDevice},
	net,
	sync::mutex::Mutex,
	utils::ptr::arc::Arc,
};

/// Vendor ID for Intel.
const VENDOR_INTEL: u16 = 0x8086;
/// Device ID for emulated NICs.
const DEVICE_EMU: u16 = 0x100e;
// TODO Add real NICs

/// The e1000 driver.
pub struct E1000Driver;

impl E1000Driver {
	/// Creates a new instance.
	pub fn new() -> Self {
		let s = Self;

		let manager = manager::get::<PCIManager>();
		if let Some(manager_mutex) = manager {
			let manager = manager_mutex.lock();
			let pci_manager = (&*manager as &dyn Any)
				.downcast_ref::<PCIManager>()
				.unwrap();

			for dev in pci_manager.get_devices() {
				s.on_plug(dev);
			}
		}

		s
	}
}

impl Driver for E1000Driver {
	fn get_name(&self) -> &str {
		"e1000"
	}

	fn on_plug(&self, dev: &dyn PhysicalDevice) {
		if dev.get_vendor_id() != VENDOR_INTEL {
			return;
		}

		match dev.get_device_id() {
			// TODO Add real NICs
			DEVICE_EMU => {
				// TODO support devices with multiple interfaces
				match NIC::new(dev) {
					Ok(nic) => {
						// TODO do not unwrap errors
						// TODO figure out how to get the name of the interface
						let name = b"TODO".try_into().unwrap();
						let iface = Arc::new(Mutex::new(nic)).unwrap();

						let mut ifaces = net::INTERFACES.lock();
						ifaces.insert(name, iface).unwrap();
					}

					Err(e) => {
						kernel::println!("e1000 error: {e}");
					}
				}
			}

			_ => {}
		}
	}

	fn on_unplug(&self, _dev: &dyn PhysicalDevice) {
		todo!()
	}
}
