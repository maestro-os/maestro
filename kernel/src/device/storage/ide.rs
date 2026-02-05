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

//! The Integrated Drive Electronics (IDE) is a controller allowing to access
//! storage drives.

use crate::device::{
	BlkDev, DeviceID,
	bar::Bar,
	register_blk,
	storage::{
		PhysicalDevice, SCSI_MAJOR, STORAGE_MODE, partition::read_partitions, pata::PATAInterface,
	},
};
use core::{
	num::NonZeroU64,
	sync::atomic::{AtomicU32, Ordering::Relaxed},
};
use utils::{boxed::Box, collections::path::PathBuf, errno::EResult, format};

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
	pub ata_bar: Bar,
	/// The BAR for control port.
	pub control_bar: Bar,
}

impl Channel {
	/// Returns a new instance representing the channel in compatibility mode.
	///
	/// `secondary` tells whether the primary or secondary channel is picked.
	pub fn new_compatibility(secondary: bool) -> Self {
		if secondary {
			Self {
				ata_bar: Bar::IOSpace {
					address: SECONDARY_ATA_BUS_PORT_BEGIN as _,
					size: 8,
				},
				control_bar: Bar::IOSpace {
					address: SECONDARY_DEVICE_CONTROL_PORT as _,
					size: 4,
				},
			}
		} else {
			Self {
				ata_bar: Bar::IOSpace {
					address: PRIMARY_ATA_BUS_PORT_BEGIN as _,
					size: 8,
				},
				control_bar: Bar::IOSpace {
					address: PRIMARY_DEVICE_CONTROL_PORT as _,
					size: 4,
				},
			}
		}
	}
}

/// An IDE controller.
#[derive(Debug)]
pub struct Controller {
	/// Programming Interface Byte
	prog_if: u8,
	/// IDE controller's BARs
	bars: [Option<Bar>; 5],
}

impl Controller {
	/// Creates a new instance.
	pub fn new(dev: &dyn PhysicalDevice) -> EResult<Self> {
		let bars = dev.get_bars();
		let ctrlr = Self {
			prog_if: dev.get_prog_if(),
			bars: [
				bars[0].clone(),
				bars[1].clone(),
				bars[2].clone(),
				bars[3].clone(),
				bars[4].clone(),
			],
		};
		for i in 0..4 {
			let secondary = (i & 0b10) != 0;
			let slave = (i & 0b01) != 0;
			let pci_mode = (!secondary && ctrlr.is_primary_pci_mode())
				|| (secondary && ctrlr.is_secondary_pci_mode());
			let channel = if pci_mode {
				if !secondary {
					// Primary channel
					Channel {
						ata_bar: ctrlr.bars[0].clone().unwrap(),
						control_bar: ctrlr.bars[1].clone().unwrap(),
					}
				} else {
					// Secondary channel
					Channel {
						ata_bar: ctrlr.bars[2].clone().unwrap(),
						control_bar: ctrlr.bars[3].clone().unwrap(),
					}
				}
			} else {
				// Compatibility mode
				Channel::new_compatibility(secondary)
			};
			// Assign disk ID
			static ID: AtomicU32 = AtomicU32::new(0);
			let scsi_id = ID.fetch_add(1, Relaxed);
			let Some(interface) = PATAInterface::new(scsi_id, channel, slave) else {
				continue;
			};
			// Prefix is the path of the main device file
			// TODO Handle if out of the alphabet
			let letter = (b'a' + scsi_id as u8) as char;
			let path = PathBuf::new_unchecked(format!("/dev/sd{letter}")?);
			// Register devices
			let dev = BlkDev::new(
				DeviceID {
					major: SCSI_MAJOR,
					minor: scsi_id * 16,
				},
				path,
				STORAGE_MODE,
				NonZeroU64::new(512).unwrap(),
				interface.sectors_count,
				Box::new(interface)?,
			)?;
			register_blk(dev.clone())?;
			read_partitions(&dev)?;
		}
		Ok(ctrlr)
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
}
