//! The Integrated Drive Electronics (IDE) is a controller allowing to access
//! storage drives.

use crate::device::bar::BAR;
use crate::device::bus::pci;
use crate::device::storage::pata::PATAInterface;
use crate::device::storage::PhysicalDevice;
use crate::device::storage::StorageInterface;
use crate::errno::AllocResult;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;

/// The beginning of the port range for the primary ATA bus (compatibility
/// mode).
const PRIMARY_ATA_BUS_PORT_BEGIN: u16 = 0x1f0;
/// The port for the primary disk's device control register (compatibility
/// mode).
const PRIMARY_DEVICE_CONTROL_PORT: u16 = 0x3f6;
/// The port for the primary disk's alternate status register (compatibility
/// mode).
const PRIMARY_ALTERNATE_STATUS_PORT: u16 = 0x3f6;

/// The beginning of the port range for the secondary ATA bus (compatibility
/// mode).
const SECONDARY_ATA_BUS_PORT_BEGIN: u16 = 0x170;
/// The port for the secondary disk's device control register (compatibility
/// mode).
const SECONDARY_DEVICE_CONTROL_PORT: u16 = 0x376;
/// The port for the secondary disk's alternate status register (compatibility
/// mode).
const SECONDARY_ALTERNATE_STATUS_PORT: u16 = 0x376;

/// Structure representing a channel on an IDE controller. It contains the BARs
/// used to access a drive.
#[derive(Debug)]
pub struct Channel {
	/// The BAR for ATA ports.
	pub ata_bar: BAR,
	/// The BAR for control port.
	pub control_bar: BAR,
}

impl Channel {
	/// Returns a new instance representing the channel in compatibility mode.
	///
	/// `secondary` tells whether the primary or secondary channel is picked.
	pub fn new_compatibility(secondary: bool) -> Self {
		if secondary {
			Self {
				ata_bar: BAR::IOSpace {
					address: SECONDARY_ATA_BUS_PORT_BEGIN as _,

					size: 8,
				},
				control_bar: BAR::IOSpace {
					address: SECONDARY_DEVICE_CONTROL_PORT as _,

					size: 4,
				},
			}
		} else {
			Self {
				ata_bar: BAR::IOSpace {
					address: PRIMARY_ATA_BUS_PORT_BEGIN as _,

					size: 8,
				},
				control_bar: BAR::IOSpace {
					address: PRIMARY_DEVICE_CONTROL_PORT as _,

					size: 4,
				},
			}
		}
	}
}

/// Structure representing an IDE controller.
#[derive(Debug)]
pub struct Controller {
	/// Programming Interface Byte
	prog_if: u8,

	/// IDE controller's BARs.
	bars: [Option<BAR>; 5],
}

impl Controller {
	/// Creates a new instance from the given `PhysicalDevice`.
	///
	/// If the given device is not an IDE controller, the function returns `None`.
	pub fn new(dev: &dyn PhysicalDevice) -> Option<Self> {
		if dev.get_class() != pci::CLASS_MASS_STORAGE_CONTROLLER || dev.get_subclass() != 0x01 {
			return None;
		}

		let bars = dev.get_bars();
		Some(Self {
			prog_if: dev.get_prog_if(),

			bars: [
				bars[0].clone(),
				bars[1].clone(),
				bars[2].clone(),
				bars[3].clone(),
				bars[4].clone(),
			],
		})
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

	/// Detects all disks on the controller. For each disks, the function calls
	/// the given closure `f`.
	///
	/// If an error is returned from a call to the closure, the function returns
	/// with the same error.
	pub(super) fn detect(
		&self,
	) -> impl '_ + Iterator<Item = AllocResult<Arc<Mutex<dyn StorageInterface>>>> {
		(0..4)
			.map(|i| {
				let secondary = (i & 0b10) != 0;
				let slave = (i & 0b01) != 0;
				let pci_mode = (!secondary && self.is_primary_pci_mode())
					|| (secondary && self.is_secondary_pci_mode());
				if !pci_mode {
					// Compatibility mode
					return (Channel::new_compatibility(secondary), slave);
				}
				let channel = if !secondary {
					// Primary channel
					Channel {
						ata_bar: self.bars[0].clone().unwrap(),
						control_bar: self.bars[1].clone().unwrap(),
					}
				} else {
					// Secondary channel
					Channel {
						ata_bar: self.bars[2].clone().unwrap(),
						control_bar: self.bars[3].clone().unwrap(),
					}
				};
				(channel, slave)
			})
			// TODO log errors?
			.filter_map(|(channel, slave)| PATAInterface::new(channel, slave).ok())
			.map(|i| Arc::new(Mutex::new(i)).map(|a| a as Arc<Mutex<dyn StorageInterface>>))
	}
}
