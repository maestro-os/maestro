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

//! This module implements the PATA interface for hard drives.
//!
//! The PATA interface is an old, deprecated interface that has been replaced by
//! the SATA interface.
//!
//! ATA devices may be detected by the PCI, but if not, it doesn't mean that
//! they are not present. The disk(s) may instead use the standardized IO ports
//! for legacy support.
//!
//! Legacy PATA can support up to two buses, each supporting up to two drives.
//! Each bus has its own set of ports.
//!
//! Before using a drive, the kernel has to:
//! - Reset the ATA controller
//! - Select the drive (with the dedicated command)
//! - Identify it to retrieve information, such as whether the drives support LBA48
//!
//! TODO

// TODO Add support for third and fourth bus

use super::StorageInterface;
use crate::{device::storage::ide, io};
use core::{cmp::min, num::NonZeroU64};
use utils::{errno, errno::EResult};

/// Offset to the data register.
const DATA_REGISTER_OFFSET: u16 = 0;
/// Offset to the error register.
const ERROR_REGISTER_OFFSET: u16 = 1;
/// Offset to the features register.
const FEATURES_REGISTER_OFFSET: u16 = 1;
/// Offset to the sectors count register.
const SECTORS_COUNT_REGISTER_OFFSET: u16 = 2;
/// Offset to the LBA low register.
const LBA_LO_REGISTER_OFFSET: u16 = 3;
/// Offset to the LBA mid register.
const LBA_MID_REGISTER_OFFSET: u16 = 4;
/// Offset to the LBA high register.
const LBA_HI_REGISTER_OFFSET: u16 = 5;
/// Offset to the drive register.
const DRIVE_REGISTER_OFFSET: u16 = 6;
/// Offset to the status register.
const STATUS_REGISTER_OFFSET: u16 = 7;
/// Offset to the command register.
const COMMAND_REGISTER_OFFSET: u16 = 7;

/// Selects the master drive.
const SELECT_MASTER: u8 = 0xa0;
/// Selects the slave drive.
const SELECT_SLAVE: u8 = 0xb0;

/// Reads sectors from the disk with LBA28.
const COMMAND_READ_SECTORS: u8 = 0x20;
/// Reads sectors from the disk with LBA48.
const COMMAND_READ_SECTORS_EXT: u8 = 0x24;
/// Writes sectors on the disk with LBA28.
const COMMAND_WRITE_SECTORS: u8 = 0x30;
/// Writes sectors on the disk with LBA48.
const COMMAND_WRITE_SECTORS_EXT: u8 = 0x34;
/// Flush cache command.
const COMMAND_CACHE_FLUSH: u8 = 0xe7;
/// Identifies the selected drive.
const COMMAND_IDENTIFY: u8 = 0xec;

/// Address mark not found.
const ERROR_AMNF: u8 = 0b00000001;
/// Track zero not found.
const ERROR_TKZNF: u8 = 0b00000010;
/// Aborted command.
const ERROR_ABRT: u8 = 0b00000100;
/// Media change request.
const ERROR_MCR: u8 = 0b00001000;
/// ID not found.
const ERROR_IDNF: u8 = 0b00010000;
/// Media changed.
const ERROR_MC: u8 = 0b00100000;
/// Uncorrectable data error.
const ERROR_UNC: u8 = 0b01000000;
/// Bad block detected.
const ERROR_BBK: u8 = 0b10000000;

/// Indicates an error occurred.
const STATUS_ERR: u8 = 0b00000001;
/// Set when drive has PIO data to transfer or is ready to accept PIO data.
const STATUS_DRQ: u8 = 0b00001000;
/// Overlapped Mode Service Request.
const STATUS_SRV: u8 = 0b00010000;
/// Drive Fault Error.
const STATUS_DF: u8 = 0b00100000;
/// Clear after an error. Set otherwise.
const STATUS_RDY: u8 = 0b01000000;
/// Indicates the drive is preparing to send/receive data.
const STATUS_BSY: u8 = 0b10000000;

/// The size of a sector in bytes.
const SECTOR_SIZE: u64 = 512;

// TODO Synchronize both master and slave disks so that another thread cannot
// trigger a select while operating on a drive

/// Applies a delay. `n` determines the amount to wait.
///
/// This function is a dirty hack and the actual delay is approximate but
/// **should** be sufficient.
fn delay(n: u32) {
	let n = n.div_ceil(30) * 1000;
	for _ in 0..n {
		unsafe {
			io::inb(STATUS_REGISTER_OFFSET);
		}
	}
}

/// An enumeration representing port offset types for ATA.
enum PortOffset {
	/// Port offset on general register ports.
	Ata(u16),
	/// Port offset on control register ports.
	Control(u16),
}

/// Structure representing a PATA interface. An instance is associated with a
/// unique disk.
#[derive(Debug)]
pub struct PATAInterface {
	/// The channel on which the disk is located.
	channel: ide::Channel,
	/// Tells whether the disk is slave or master.
	slave: bool,

	/// Tells whether the drive supports LBA48.
	lba48: bool,

	/// The number of sectors on the disk.
	sectors_count: u64,
}

impl PATAInterface {
	/// Creates a new instance.
	///
	/// On error, the function returns a string telling the cause.
	///
	/// Arguments:
	/// - `channel` is the IDE channel of the disk.
	/// - `slave` tells whether the disk is the slave disk.
	pub fn new(channel: ide::Channel, slave: bool) -> Result<Self, &'static str> {
		let mut s = Self {
			channel,
			slave,

			lba48: false,

			sectors_count: 0,
		};
		s.identify()?;
		Ok(s)
	}

	/// Reads a byte from the register at offset `port_off`.
	#[inline(always)]
	fn inb(&self, port_off: PortOffset) -> u8 {
		let (bar, off) = match port_off {
			PortOffset::Ata(off) => (&self.channel.ata_bar, off),
			PortOffset::Control(off) => (&self.channel.control_bar, off),
		};
		bar.read::<u8>(off as _) as _
	}

	/// Reads a word from the register at offset `port_off`.
	#[inline(always)]
	fn inw(&self, port_off: PortOffset) -> u16 {
		let (bar, off) = match port_off {
			PortOffset::Ata(off) => (&self.channel.ata_bar, off),
			PortOffset::Control(off) => (&self.channel.control_bar, off),
		};
		bar.read::<u16>(off as _) as _
	}

	/// Writes a byte into the register at offset `port_off`.
	#[inline(always)]
	fn outb(&self, port_off: PortOffset, value: u8) {
		let (bar, off) = match port_off {
			PortOffset::Ata(off) => (&self.channel.ata_bar, off),
			PortOffset::Control(off) => (&self.channel.control_bar, off),
		};
		bar.write::<u8>(off as _, value as _) as _
	}

	/// Writes a word into the register at offset `port_off`.
	#[inline(always)]
	fn outw(&self, port_off: PortOffset, value: u16) {
		let (bar, off) = match port_off {
			PortOffset::Ata(off) => (&self.channel.ata_bar, off),
			PortOffset::Control(off) => (&self.channel.control_bar, off),
		};
		bar.write::<u16>(off as _, value as _) as _
	}

	/// Returns the content of the error register.
	fn get_error(&self) -> u8 {
		self.inb(PortOffset::Ata(ERROR_REGISTER_OFFSET))
	}

	/// Returns the content of the status register.
	fn get_status(&self) -> u8 {
		self.inb(PortOffset::Ata(STATUS_REGISTER_OFFSET))
	}

	/// Tells whether the device is ready to accept a command.
	fn is_ready(&self) -> bool {
		self.get_status() & STATUS_RDY != 0
	}

	/// Tells whether the disk's buses are floating.
	///
	/// A floating bus means there is no hard drive connected.
	///
	/// However, if the bus isn't floating, it doesn't necessarily mean there is a disk.
	fn is_floating(&self) -> bool {
		self.get_status() == 0xff
	}

	/// Waits until the drive is not busy anymore.
	///
	/// If the drive wasn't busy, the function doesn't do anything.
	fn wait_busy(&self) {
		if self.is_floating() {
			return;
		}
		while self.get_status() & STATUS_BSY != 0 {}
	}

	/// Sends the given command on the bus.
	///
	/// The function doesn't check if the drive is ready since it can allow to select the drive.
	///
	/// `command` is the command.
	fn send_command(&self, command: u8) {
		self.outb(PortOffset::Ata(COMMAND_REGISTER_OFFSET), command);
	}

	/// Selects the drive.
	///
	/// This operation is necessary in order to send command to the drive.
	///
	/// If the drive is already selected, the function does nothing unless `init` is set.
	fn select(&self, init: bool) {
		if !init {
			// TODO Select if necessary
			return;
		}

		let value = if !self.slave {
			SELECT_MASTER
		} else {
			SELECT_SLAVE
		};
		self.outb(PortOffset::Ata(DRIVE_REGISTER_OFFSET), value);

		delay(420);
	}

	/// Flushes the drive's cache. The device is assumed to be selected.
	fn cache_flush(&self) {
		self.send_command(COMMAND_CACHE_FLUSH);
		self.wait_busy();
	}

	/// Resets both master and slave devices.
	///
	/// The current drive may not be selected anymore after this function returns.
	fn reset(&self) {
		self.outb(PortOffset::Control(0), 1 << 2);
		delay(5000);

		self.outb(PortOffset::Control(0), 0);
		delay(5000);
	}

	/// Identifies the drive, retrieving informations about the drive.
	///
	/// On error, the function returns a string telling the cause.
	fn identify(&mut self) -> Result<(), &'static str> {
		self.reset();
		self.select(true);

		if self.is_floating() {
			return Err("Drive doesn't exist");
		}

		self.outb(PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET), 0);
		self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), 0);
		self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), 0);
		self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), 0);
		delay(420);

		self.send_command(COMMAND_IDENTIFY);
		delay(420);

		let status = self.get_status();
		if status == 0 {
			return Err("Drive doesn't exist");
		}
		self.wait_busy();

		let lba_mid = self.inb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET));
		let lba_hi = self.inb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET));

		if lba_mid != 0 || lba_hi != 0 {
			return Err("Unknown device");
		}

		loop {
			let status = self.get_status();
			if status & STATUS_ERR != 0 {
				return Err("Error while identifying the device");
			}
			if status & STATUS_DRQ != 0 {
				break;
			}
		}

		let mut data: [u16; 256] = [0; 256];
		for d in data.iter_mut() {
			*d = self.inw(PortOffset::Ata(DATA_REGISTER_OFFSET));
		}

		// Retrieve disk size
		let lba48_support = data[83] & (1 << 10) != 0;
		let lba28_size = (data[60] as u32) | ((data[61] as u32) << 16);
		let lba48_size = (data[100] as u64)
			| ((data[101] as u64) << 16)
			| ((data[102] as u64) << 32)
			| ((data[103] as u64) << 48);
		if lba28_size == 0 {
			return Err("Unsupported disk (too old)");
		}
		self.lba48 = lba48_support;
		self.sectors_count = if lba48_support {
			lba48_size
		} else {
			lba28_size as _
		};

		delay(420);
		Ok(())
	}

	/// Waits for the drive to be ready for IO operation.
	///
	/// The device is assumed to be selected.
	fn wait_io(&self) -> EResult<()> {
		loop {
			let status = self.get_status();
			if (status & STATUS_BSY == 0) && (status & STATUS_DRQ != 0) {
				return Ok(());
			}
			if (status & STATUS_ERR != 0) || (status & STATUS_DF != 0) {
				return Err(errno!(EIO));
			}
		}
	}
}

impl StorageInterface for PATAInterface {
	fn get_block_size(&self) -> NonZeroU64 {
		SECTOR_SIZE.try_into().unwrap()
	}

	fn get_blocks_count(&self) -> u64 {
		self.sectors_count
	}

	// TODO clean
	fn read(&mut self, buf: &mut [u8], offset: u64, size: u64) -> EResult<()> {
		debug_assert!((buf.len() as u64) >= size * SECTOR_SIZE);

		// If the offset and size are out of bounds of the disk, return an error
		if offset >= self.sectors_count || offset + size > self.sectors_count {
			return Err(errno!(EINVAL));
		}

		// Tells whether to use LBA48
		let lba48 = (offset + size) >= ((1 << 28) - 1);

		// If LBA48 is required but not supported, return an error
		if lba48 && !self.lba48 {
			return Err(errno!(EIO));
		}

		// The maximum number of sectors that can be handled at each iterations
		let iter_max = if lba48 {
			(u16::MAX as u64) + 1
		} else {
			(u8::MAX as u64) + 1
		};

		self.select(false);

		let mut i = 0;
		while i < size {
			let off = offset + i;

			// The number of blocks for this iteration
			let mut count = min(size - i, iter_max);
			if count == iter_max {
				count = 0;
			}

			let mut drive = if lba48 {
				// LBA48
				0x40
			} else {
				// LBA28
				0xe0
			};
			if self.slave {
				// Setting slave bit
				drive |= 1 << 4;
			}

			// If LBA28, add the end of the sector offset
			if !lba48 {
				drive |= ((off >> 24) & 0x0f) as u8;
			}

			self.outb(PortOffset::Ata(DRIVE_REGISTER_OFFSET), drive);

			// If LBA48, write high bytes first
			if lba48 {
				let count = ((count >> 8) & 0xff) as u8;
				let lo_lba = ((off >> 24) & 0xff) as u8;
				let mid_lba = ((off >> 32) & 0xff) as u8;
				let hi_lba = ((off >> 40) & 0xff) as u8;

				self.outb(PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET), count);
				self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), lo_lba);
				self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), mid_lba);
				self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), hi_lba);
			}

			let lo_lba = (off & 0xff) as u8;
			let mid_lba = ((off >> 8) & 0xff) as u8;
			let hi_lba = ((off >> 16) & 0xff) as u8;

			self.outb(
				PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET),
				(count & 0xff) as u8,
			);
			self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), lo_lba);
			self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), mid_lba);
			self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), hi_lba);

			if lba48 {
				self.send_command(COMMAND_READ_SECTORS_EXT);
			} else {
				self.send_command(COMMAND_READ_SECTORS);
			}

			if count == 0 {
				count = iter_max;
			}

			for j in 0..count {
				self.wait_io()?;

				for k in 0..256 {
					let index = (((i + j) * 256 + k) * 2) as usize;
					debug_assert!(index + 1 < buf.len());

					let word = self.inw(PortOffset::Ata(DATA_REGISTER_OFFSET));
					buf[index] = (word & 0xff) as _;
					buf[index + 1] = ((word >> 8) & 0xff) as _;
				}
			}

			i += count;
		}

		Ok(())
	}

	// TODO clean
	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> EResult<()> {
		debug_assert!((buf.len() as u64) >= size * SECTOR_SIZE);

		// If the offset and size are out of bounds of the disk, return an error
		if offset >= self.sectors_count || offset + size > self.sectors_count {
			return Err(errno!(EINVAL));
		}

		// Tells whether to use LBA48
		let lba48 = (offset + size) >= ((1 << 28) - 1);

		// If LBA48 is required but not supported, return an error
		if lba48 && !self.lba48 {
			return Err(errno!(EIO));
		}

		// The maximum number of sectors that can be handled at each iterations
		let iter_max = if lba48 {
			(u16::MAX as u64) + 1
		} else {
			(u8::MAX as u64) + 1
		};

		self.select(false);

		let mut i = 0;
		while i < size {
			let off = offset + i;

			// The number of blocks for this iteration
			let mut count = min(size - i, iter_max);
			if count == iter_max {
				count = 0;
			}

			let mut drive = if lba48 {
				// LBA48
				0x40
			} else {
				// LBA28
				0xe0
			};
			if self.slave {
				// Setting slave bit
				drive |= 1 << 4;
			}

			// If LBA28, add the end of the sector offset
			if !lba48 {
				drive |= ((off >> 24) & 0x0f) as u8;
			}

			self.outb(PortOffset::Ata(DRIVE_REGISTER_OFFSET), drive);

			// If LBA48, write high bytes first
			if lba48 {
				let count = ((count >> 8) & 0xff) as u8;
				let lo_lba = ((off >> 24) & 0xff) as u8;
				let mid_lba = ((off >> 32) & 0xff) as u8;
				let hi_lba = ((off >> 40) & 0xff) as u8;

				self.outb(PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET), count);
				self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), lo_lba);
				self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), mid_lba);
				self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), hi_lba);
			}

			let lo_lba = (off & 0xff) as u8;
			let mid_lba = ((off >> 8) & 0xff) as u8;
			let hi_lba = ((off >> 16) & 0xff) as u8;

			self.outb(
				PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET),
				(count & 0xff) as u8,
			);
			self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), lo_lba);
			self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), mid_lba);
			self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), hi_lba);

			if lba48 {
				self.send_command(COMMAND_WRITE_SECTORS_EXT);
			} else {
				self.send_command(COMMAND_WRITE_SECTORS);
			}

			if count == 0 {
				count = iter_max;
			}

			for j in 0..count {
				self.wait_io()?;

				for k in 0..256 {
					let index = (((i + j) * 256 + k) * 2) as usize;
					debug_assert!(index + 1 < buf.len());

					let word = ((buf[index + 1] as u16) << 8) | (buf[index] as u16);
					self.outw(PortOffset::Ata(DATA_REGISTER_OFFSET), word)
				}
			}

			self.cache_flush();
			i += count;
		}

		Ok(())
	}
}
