//! This module implements internal buses, including PCI and USB.

pub mod pci;

use crate::device::manager;
use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;

/// The list of buses connected to the CPU.
static mut BUSES: Mutex<Vec<Box<dyn Bus>>> = Mutex::new(Vec::new());

/// Trait representing a bus.
pub trait Bus {
	/// Returns the name of the bus.
	fn get_name(&self) -> &str;

	/// Tells whether the bus is a hotplug bus.
	fn is_hotplug(&self) -> bool;

	// TODO
}

/// Returns the list of buses connected to the system.
pub fn get_buses() -> &'static mut Mutex<Vec<Box<dyn Bus>>> {
	unsafe { // Safe because using Mutex
		&mut BUSES
	}
}

// TODO Function to add a bus

// TODO Function to get a bus with given name

/// Detects internal buses and registers them.
pub fn detect() -> Result<(), Errno> {
	let mut pci_manager = pci::PCIManager {};
	for device in pci_manager.scan().iter() {
		manager::on_plug(device);
	}

	let mutex = unsafe {
		&mut BUSES
	};
	let mut guard = mutex.lock(true);
	let buses = guard.get_mut();
	buses.push(Box::new(pci_manager)?)
}
