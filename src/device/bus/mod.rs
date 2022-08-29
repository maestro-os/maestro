//! This module implements internal buses, including PCI and USB.

pub mod pci;

use crate::device::manager;
use crate::errno::Errno;

/// Detects internal buses and registers them.
pub fn detect() -> Result<(), Errno> {
	// PCI
	let mut pci_manager = pci::PCIManager::new();
	pci_manager.scan()?;
	manager::register_manager(pci_manager)?;

	// TODO USB

	Ok(())
}
