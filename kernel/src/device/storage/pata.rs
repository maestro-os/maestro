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

//! PATA interface implementation, for hard drives I/O.
//!
//! The PATA interface is an old, deprecated interface implemented for backward compatibility.
//!
//! ATA devices may be detected by the PCI, but if not, it does not mean that
//! they are not present. The disk(s) may instead use the standardized IO ports
//! for legacy support.
//!
//! Legacy PATA can support up to two buses, each supporting up to two drives.
//! Each bus has its own set of ports.

// TODO Add support for third and fourth bus

use crate::{
	arch::x86::io::inb,
	device::{storage::ide, BlockDeviceOps},
	memory::{
		buddy::{FrameOrder, ZONE_KERNEL},
		cache::{FrameOwner, RcFrame},
	},
	sync::mutex::Mutex,
};
use core::{intrinsics::unlikely, num::NonZeroU64};
use utils::{bytes::slice_from_bytes, errno, errno::EResult, limits::PAGE_SIZE};

/// Offset to the data register
const DATA_REGISTER_OFFSET: u16 = 0;
/// Offset to the error register
const ERROR_REGISTER_OFFSET: u16 = 1;
/// Offset to the features register
const FEATURES_REGISTER_OFFSET: u16 = 1;
/// Offset to the sectors count register
const SECTORS_COUNT_REGISTER_OFFSET: u16 = 2;
/// Offset to the LBA low register
const LBA_LO_REGISTER_OFFSET: u16 = 3;
/// Offset to the LBA mid register
const LBA_MID_REGISTER_OFFSET: u16 = 4;
/// Offset to the LBA high register
const LBA_HI_REGISTER_OFFSET: u16 = 5;
/// Offset to the drive register
const DRIVE_REGISTER_OFFSET: u16 = 6;
/// Offset to the status register
const STATUS_REGISTER_OFFSET: u16 = 7;
/// Offset to the command register
const COMMAND_REGISTER_OFFSET: u16 = 7;

/// Selects the master drive
const SELECT_MASTER: u8 = 0xa0;
/// Selects the slave drive
const SELECT_SLAVE: u8 = 0xb0;

/// Reads sectors from the disk (LBA28)
const COMMAND_READ_SECTORS: u8 = 0x20;
/// Writes sectors on the disk (LBA28)
const COMMAND_WRITE_SECTORS: u8 = 0x30;
/// Flush cache command
const COMMAND_CACHE_FLUSH: u8 = 0xe7;
/// Flush cache command (LBA48)
const COMMAND_CACHE_FLUSH_EXT: u8 = 0xea;
/// Identifies the selected drive
const COMMAND_IDENTIFY: u8 = 0xec;

/// Address mark not found
const ERROR_AMNF: u8 = 0b00000001;
/// Track zero not found
const ERROR_TKZNF: u8 = 0b00000010;
/// Aborted command
const ERROR_ABRT: u8 = 0b00000100;
/// Media change request
const ERROR_MCR: u8 = 0b00001000;
/// ID not found
const ERROR_IDNF: u8 = 0b00010000;
/// Media changed
const ERROR_MC: u8 = 0b00100000;
/// Uncorrectable data error
const ERROR_UNC: u8 = 0b01000000;
/// Bad block detected
const ERROR_BBK: u8 = 0b10000000;

/// Indicates an error occurred
const STATUS_ERR: u8 = 0b00000001;
/// Set when drive has PIO data to transfer or is ready to accept PIO data
const STATUS_DRQ: u8 = 0b00001000;
/// Overlapped Mode Service Request
const STATUS_SRV: u8 = 0b00010000;
/// Drive Fault Error
const STATUS_DF: u8 = 0b00100000;
/// Clear after an error. Set otherwise
const STATUS_RDY: u8 = 0b01000000;
/// Indicates the drive is preparing to send/receive data
const STATUS_BSY: u8 = 0b10000000;

/// The size of a sector in bytes
const SECTOR_SIZE: u64 = 512;
/// The number of sectors per page of memory
const SECTOR_PER_PAGE: u64 = PAGE_SIZE as u64 / SECTOR_SIZE;

/// Applies a delay. `n` determines the amount to wait.
///
/// This function is a dirty hack and the actual delay is approximate but
/// **should** be sufficient.
fn delay(n: u32) {
	let n = n.div_ceil(30) * 1000;
	for _ in 0..n {
		unsafe {
			inb(STATUS_REGISTER_OFFSET);
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

/// A PATA interface with a unique disk.
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

	/// Mutex preventing data race on read/write operations.
	lock: Mutex<()>,
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

			lock: Default::default(),
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
	fn cache_flush(&self, lba48: bool) {
		if lba48 {
			self.send_command(COMMAND_CACHE_FLUSH_EXT);
		} else {
			self.send_command(COMMAND_CACHE_FLUSH);
		}
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

	/// Prepare for an I/O operation.
	///
	/// Arguments:
	/// - `off` is the offset of the first sector
	/// - `count` is the number of sectors on which the operation is applied
	/// - `write` tells whether this is a write operation. If `false`, this is a read operation
	///
	/// The function returns a tuple containing:
	/// - The number of sectors to write
	/// - Whether we are using LBA48
	fn prepare_io(&self, off: u64, count: u16, write: bool) -> (u16, bool) {
		// Tells whether we have to use LBA48
		let lba48 = self.lba48 && (count > u8::MAX as u16 || off + count as u64 >= 1 << 28);
		let mut drive = if lba48 { 0x40 } else { 0xe0 };
		if self.slave {
			drive |= 1 << 4;
		}
		// If LBA28, add the end of the sector offset
		if !lba48 {
			drive |= ((off >> 24) & 0xf) as u8;
		}
		self.outb(PortOffset::Ata(DRIVE_REGISTER_OFFSET), drive);
		// Write sectors count and offset
		let max = if lba48 { u16::MAX } else { u8::MAX as u16 };
		let count = count.min(max);
		if lba48 {
			self.outb(
				PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET),
				(count >> 8) as u8,
			);
			self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), (off >> 24) as u8);
			self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), (off >> 32) as u8);
			self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), (off >> 40) as u8);
		}
		self.outb(PortOffset::Ata(SECTORS_COUNT_REGISTER_OFFSET), count as u8);
		self.outb(PortOffset::Ata(LBA_LO_REGISTER_OFFSET), off as u8);
		self.outb(PortOffset::Ata(LBA_MID_REGISTER_OFFSET), (off >> 8) as u8);
		self.outb(PortOffset::Ata(LBA_HI_REGISTER_OFFSET), (off >> 16) as u8);
		// Send command
		let mut cmd = if !write {
			COMMAND_READ_SECTORS
		} else {
			COMMAND_WRITE_SECTORS
		};
		if lba48 {
			cmd |= 0x4;
		}
		self.send_command(cmd);
		(count, lba48)
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

impl BlockDeviceOps for PATAInterface {
	fn block_size(&self) -> NonZeroU64 {
		SECTOR_SIZE.try_into().unwrap()
	}

	fn blocks_count(&self) -> u64 {
		self.sectors_count
	}

	fn read_frame(&self, off: u64, order: FrameOrder, owner: FrameOwner) -> EResult<RcFrame> {
		let frame = RcFrame::new(order, ZONE_KERNEL, owner, off)?;
		let off = off
			.checked_mul(SECTOR_PER_PAGE)
			.ok_or_else(|| errno!(EOVERFLOW))?;
		let size = frame.pages_count() as u64 * SECTOR_PER_PAGE;
		// If the offset and size are out of bounds of the disk, return an error
		let end = off.checked_add(size).ok_or_else(|| errno!(EOVERFLOW))?;
		if end > self.sectors_count {
			return Err(errno!(EOVERFLOW));
		}
		// Avoid data race
		let _guard = self.lock.lock();
		// Select disk
		self.select(false);
		// Read
		let buf = unsafe { frame.slice_mut() };
		let mut i = 0;
		while i < size {
			let off = off + i;
			let count = (size - i).min(u16::MAX as u64) as u16;
			let (count, _) = self.prepare_io(off, count, false);
			let start = i as usize;
			let end = start + count as usize;
			for j in start..end {
				self.wait_io()?;
				for k in 0..256 {
					let index = j * 256 + k;
					buf[index] = self.inw(PortOffset::Ata(DATA_REGISTER_OFFSET));
				}
			}
			i += count as u64;
		}
		Ok(frame)
	}

	fn write_pages(&self, off: u64, buf: &[u8]) -> EResult<()> {
		if unlikely(buf.len() % PAGE_SIZE != 0) {
			return Err(errno!(EINVAL));
		}
		let off = off
			.checked_mul(SECTOR_PER_PAGE)
			.ok_or_else(|| errno!(EOVERFLOW))?;
		let size = buf.len() as u64 / SECTOR_SIZE;
		// If the offset and size are out of bounds of the disk, return an error
		let end = off.checked_add(size).ok_or_else(|| errno!(EOVERFLOW))?;
		if end > self.sectors_count {
			return Err(errno!(EOVERFLOW));
		}
		// Avoid data race
		let _guard = self.lock.lock();
		// Select disk
		self.select(false);
		// Write
		let buf = slice_from_bytes::<u16>(buf).unwrap();
		let mut i = 0;
		while i < size {
			let off = off + i;
			let count = (size - i).min(u16::MAX as u64) as u16;
			let (count, lba48) = self.prepare_io(off, count, true);
			let start = i as usize;
			let end = start + count as usize;
			for j in start..end {
				self.wait_io()?;
				for k in 0..256 {
					let index = j * 256 + k;
					self.outw(PortOffset::Ata(DATA_REGISTER_OFFSET), buf[index])
				}
			}
			self.cache_flush(lba48);
			i += count as u64;
		}
		Ok(())
	}
}
