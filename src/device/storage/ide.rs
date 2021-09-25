//! The Integrated Drive Electronics (IDE) is a controller allowing to access storage drives.

use crate::device::bar::BAR;
use crate::device::bus::pci;
use crate::device::storage::PhysicalDevice;
use crate::device::storage::StorageInterface;
use crate::device::storage::pata::PATAInterface;
use crate::errno::Errno;
use crate::io;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;

/// The beginning of the port range for the primary ATA bus.
const PRIMARY_ATA_BUS_PORT_BEGIN: u16 = 0x1f0;
/// The port for the primary disk's device control register.
const PRIMARY_DEVICE_CONTROL_PORT: u16 = 0x3f6;
/// The port for the primary disk's alternate status register.
const PRIMARY_ALTERNATE_STATUS_PORT: u16 = 0x3f6;

/// The beginning of the port range for the secondary ATA bus.
const SECONDARY_ATA_BUS_PORT_BEGIN: u16 = 0x170;
/// The port for the secondary disk's device control register.
const SECONDARY_DEVICE_CONTROL_PORT: u16 = 0x376;
/// The port for the secondary disk's alternate status register.
const SECONDARY_ALTERNATE_STATUS_PORT: u16 = 0x376;

/// Structure representing an IDE controller.
pub struct IDEController {
	/// TODO doc
	prog_if: u8,

	/// Addresses to access the IDE controller.
	io_addresses: [BAR; 4],
}

impl IDEController {
	/// Creates a new instance from the given PhysicalDevice.
	/// If the given device is not an IDE controller, the behaviour is undefined.
	pub fn new(dev: &dyn PhysicalDevice) -> Self {
		debug_assert_eq!(dev.get_class(), pci::CLASS_MASS_STORAGE_CONTROLLER);
		debug_assert_eq!(dev.get_subclass(), 0x01);

		Self {
			prog_if: dev.get_prog_if(),

			io_addresses: [
				dev.get_bar(0).unwrap(),
				dev.get_bar(1).unwrap(),
				dev.get_bar(2).unwrap(),
				dev.get_bar(3).unwrap(),
			],
		}
	}

	/// Tells whether the primary bus of the controller is in PCI mode.
	#[inline(always)]
	pub fn is_primary_pci_mode(&self) -> bool {
		self.prog_if & 0b1 != 0
	}

	/// Tells whether the secondary bus of the controller is in PCI mode.
	#[inline(always)]
	pub fn is_secondary_pci_mode(&self) -> bool {
		self.prog_if & 0b100 != 0
	}

	/// Tells whether the controller supports DMA.
	#[inline(always)]
	pub fn is_dma(&self) -> bool {
		self.prog_if & 0b10000000 != 0
	}

	/// Reads the value of the register at offset `off`.
	pub fn reg_read(&self, _off: usize) -> u32 {
		// TODO
		0
	}

	/// Writes the value `val` to the register at offset `off`.
	pub fn reg_write(&self, _off: usize, _val: u32) {
		// TODO
	}

	/// Resets the given bus. The current drive may not be selected anymore.
	/// `secondary` tells which bus to reset. If set, the secondary is selected. If clear, the
	/// primary is selected.
	pub fn reset(&self, secondary: bool) {
		let port = if !secondary {
			PRIMARY_DEVICE_CONTROL_PORT
		} else {
			SECONDARY_DEVICE_CONTROL_PORT
		};

		unsafe {
			io::outb(port, 1 << 2);
		}

		// TODO self.wait(true);

		unsafe {
			io::outb(port, 0);
		}

		// TODO self.wait(true);
	}

	/// Detects a disk on the controller.
	/// `secondary` tells whether the disk is on the secondary bus.
	/// `slave` tells whether the disk is the slave disk.
	pub fn detect(&self, secondary: bool, slave: bool)
		-> Result<Option<Box<dyn StorageInterface>>, Errno> {
		// TODO Add support for DMA and SATA

		if let Ok(interface) = PATAInterface::new(secondary, slave) {
			Ok(Some(Box::new(interface)?))
		} else {
			Ok(None)
		}
	}

	/// Detects all disks on the controller. For each disks, the function calls the given closure
	/// `f`.
	/// If an error is returned from a call to the closure, the function returns with the same
	/// error.
	pub fn detect_all(&self) -> Result<Vec<Box<dyn StorageInterface>>, Errno> {
		let mut interfaces = Vec::new();

		for i in 0..4 {
			let secondary = (i & 0b10) != 0;
			let slave = (i & 0b01) != 0;

			if let Some(disk) = self.detect(secondary, slave)? {
				interfaces.push(disk)?;
			}
		}

		Ok(interfaces)
	}
}
