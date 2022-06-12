//! The Integrated Drive Electronics (IDE) is a controller allowing to access storage drives.

use crate::device::bar::BAR;
use crate::device::bus::pci;
use crate::device::storage::PhysicalDevice;
use crate::device::storage::StorageInterface;
use crate::device::storage::pata::PATAInterface;
use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;

/// The beginning of the port range for the primary ATA bus.
pub const PRIMARY_ATA_BUS_PORT_BEGIN: u16 = 0x1f0;
/// The port for the primary disk's device control register.
pub const PRIMARY_DEVICE_CONTROL_PORT: u16 = 0x3f6;
/// The port for the primary disk's alternate status register.
pub const PRIMARY_ALTERNATE_STATUS_PORT: u16 = 0x3f6;

/// The beginning of the port range for the secondary ATA bus.
pub const SECONDARY_ATA_BUS_PORT_BEGIN: u16 = 0x170;
/// The port for the secondary disk's device control register.
pub const SECONDARY_DEVICE_CONTROL_PORT: u16 = 0x376;
/// The port for the secondary disk's alternate status register.
pub const SECONDARY_ALTERNATE_STATUS_PORT: u16 = 0x376;

/// Enumeration representing ways to access the IDE channel.
#[derive(Debug)]
pub enum Channel {
	/// The disk has to be accessed through MMIO.
	MMIO {
		/// The BAR for ATA ports.
		ata_bar: BAR,
		/// The BAR for control port.
		control_bar: BAR,
	},

	/// The disk has to be accessed through port IO.
	IO {
		/// Tells whether the disk is on the secondary or primary bus.
		secondary: bool,
	},
}

/// Structure representing an IDE controller.
pub struct Controller {
	/// TODO doc
	prog_if: u8,

	/// IDE controller's BARs.
	bars: [Option<BAR>; 5],
}

impl Controller {
	/// Creates a new instance from the given PhysicalDevice.
	/// If the given device is not an IDE controller, the behaviour is undefined.
	pub fn new(dev: &dyn PhysicalDevice) -> Self {
		debug_assert_eq!(dev.get_class(), pci::CLASS_MASS_STORAGE_CONTROLLER);
		debug_assert_eq!(dev.get_subclass(), 0x01);

		Self {
			prog_if: dev.get_prog_if(),

			bars: [
				dev.get_bar(0),
				dev.get_bar(1),
				dev.get_bar(2),
				dev.get_bar(3),
				dev.get_bar(4),
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

	/// Detects a disk on the controller.
	/// `channel` is the channel to check.
	/// `slave` tells whether the disk is the slave disk.
	pub fn detect(&self, channel: Channel, slave: bool)
		-> Result<Option<SharedPtr<dyn StorageInterface>>, Errno> {
		// TODO Add support for SATA

		if let Ok(interface) = PATAInterface::new(channel, slave) {
			let interface = SharedPtr::new(interface)?;

			// Wrapping the interface into a cached interface
			// TODO Use a constant for the sectors count
			//let interface = Box::new(CachedStorageInterface::new(interface, 1024)?)?;

			Ok(Some(interface))
		} else {
			Ok(None)
		}
	}

	/// Detects all disks on the controller. For each disks, the function calls the given closure
	/// `f`.
	/// If an error is returned from a call to the closure, the function returns with the same
	/// error.
	pub fn detect_all(&self) -> Result<Vec<SharedPtr<dyn StorageInterface>>, Errno> {
		let mut interfaces = Vec::new();

		for i in 0..4 {
			let secondary = (i & 0b10) != 0;
			let slave = (i & 0b01) != 0;

			let pci_mode = (!secondary && self.is_primary_pci_mode())
				|| (secondary && self.is_secondary_pci_mode());

			let channel = if pci_mode {
				if !secondary {
					// Primary channel
					Channel::MMIO {
						ata_bar: self.bars[0].clone().unwrap(),
						control_bar: self.bars[1].clone().unwrap(),
					}
				} else {
					// Secondary channel
					Channel::MMIO {
						ata_bar: self.bars[2].clone().unwrap(),
						control_bar: self.bars[3].clone().unwrap(),
					}
				}
			} else {
				// Compatibility mode
				Channel::IO {
					secondary,
				}
			};


			if let Some(disk) = self.detect(channel, slave)? {
				interfaces.push(disk)?;
			}
		}

		Ok(interfaces)
	}
}
