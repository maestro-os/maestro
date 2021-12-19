//! This module implements the PATA interface for hard drives.
//! The PATA interface is an old, deprecated interface that has been replaced by the SATA
//! interface.
//! ATA devices may be detected by the PCI, but if not, it doesn't mean that they are not present.
//! The disk(s) may instead use the standardized IO ports for legacy support.
//!
//! Legacy PATA can support up to two buses, each supporting up to two drives.
//! Each bus has its own set of ports. Before using a drive, the kernel has to:
//! - Reset the ATA controller
//! - Select the drive (with the dedicated command)
//! - Identify it to retrieve informations, such as whether the drives support LBA48
//!
//! TODO

// TODO Add support for third and fourth bus

use crate::errno::Errno;
use crate::errno;
use crate::io;
use crate::util::math;
use super::StorageInterface;

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

/// Reads sectors from the disk.
const COMMAND_READ_SECTORS: u8 = 0x20;
/// Writes sectors on the disk.
const COMMAND_WRITE_SECTORS: u8 = 0x30;
/// Flush cache command.
const COMMAND_CACHE_FLUSH: u8 = 0xe7;
/// Identifies the selected drive.
const COMMAND_IDENTIFY: u8 = 0xec;

/// Address mark not found.
const ERROR_AMNF: u8  = 0b00000001;
/// Track zero not found.
const ERROR_TKZNF: u8 = 0b00000010;
/// Aborted command.
const ERROR_ABRT: u8  = 0b00000100;
/// Media change request.
const ERROR_MCR: u8   = 0b00001000;
/// ID not found.
const ERROR_IDNF: u8  = 0b00010000;
/// Media changed.
const ERROR_MC: u8    = 0b00100000;
/// Uncorrectable data error.
const ERROR_UNC: u8   = 0b01000000;
/// Bad block detected.
const ERROR_BBK: u8   = 0b10000000;

/// Indicates an error occurred.
const STATUS_ERR: u8 = 0b00000001;
/// Set when drive has PIO data to transfer or is ready to accept PIO data.
const STATUS_DRQ: u8 = 0b00001000;
/// Overlapped Mode Service Request.
const STATUS_SRV: u8 = 0b00010000;
/// Drive Fault Error.
const STATUS_DF: u8  = 0b00100000;
/// Clear after an error. Set otherwise.
const STATUS_RDY: u8 = 0b01000000;
/// Indicates the drive is preparing to send/receive data.
const STATUS_BSY: u8 = 0b10000000;

// TODO Synchronize both master and slave disks so that another thread cannot trigger a select
// while operating on a drive

/// Structure representing a PATA interface. An instance is associated with a unique disk.
pub struct PATAInterface {
	/// Tells whether the disk is on the secondary or primary bus.
	secondary: bool,
	/// Tells whether the disk is slave or master.
	slave: bool,

	/// Tells whether the drive supports LBA48.
	lba48: bool,

	/// Tells whether the drive is ATAPI.
	atapi: bool,
	/// Tells whether the drive is SATA.
	sata: bool,

	/// The number of sectors on the disk.
	sectors_count: u64,
}

impl PATAInterface {
	/// Creates a new instance. On error, the function returns a string telling the cause.
	/// `secondary` tells whether the disk is on the secondary bus.
	/// `slave` tells whether the disk is the slave disk.
	pub fn new(secondary: bool, slave: bool) -> Result<Self, &'static str> {
		let mut s = Self {
			secondary,
			slave,

			lba48: false,

			atapi: false,
			sata: false,

			sectors_count: 0,
		};
		s.identify()?;
		Ok(s)
	}

	/// Tells whether the interface is for the secondary bus.
	pub fn is_secondary(&self) -> bool {
		self.secondary
	}

	/// Tells whether the interface is for the slave disk.
	pub fn is_slave(&self) -> bool {
		self.slave
	}

	/// Tells whether the drive supports LBA48.
	pub fn supports_lba48(&self) -> bool {
		self.lba48
	}

	/// Tells whether the drive is ATAPI.
	pub fn is_atapi(&self) -> bool {
		self.atapi
	}

	/// Tells whether the drive is SATA.
	pub fn is_sata(&self) -> bool {
		self.sata
	}

	/// Returns the port for the register at offset `offset`.
	fn get_register_port(&self, offset: u16) -> u16 {
		(if !self.secondary {
			PRIMARY_ATA_BUS_PORT_BEGIN
		} else {
			SECONDARY_ATA_BUS_PORT_BEGIN
		}) + offset
	}

	/// Returns the content of the error register.
	fn get_error(&self) -> u8 {
		let port = self.get_register_port(ERROR_REGISTER_OFFSET);
		unsafe {
			io::inb(port)
		}
	}

	/// Returns the content of the status register.
	fn get_status(&self) -> u8 {
		let port = self.get_register_port(STATUS_REGISTER_OFFSET);
		unsafe {
			io::inb(port)
		}
	}

	/// Tells whether the device is ready to accept a command.
	fn is_ready(&self) -> bool {
		self.get_status() & STATUS_RDY != 0
	}

	/// Waits until the drive is not busy anymore. If the drive wasn't busy, the function doesn't
	/// do anything.
	fn wait_busy(&self) {
		while self.get_status() & STATUS_BSY != 0 {}
	}

	/// Sends the given command on the bus. The function doesn't check if the drive is ready since
	/// it can allow to select the drive.
	/// `command` is the command.
	fn send_command(&self, command: u8) {
		let port = self.get_register_port(COMMAND_REGISTER_OFFSET);
		unsafe {
			io::outb(port, command);
		}
	}

	/// Selects the drive. This operation is necessary in order to send command to the drive.
	/// NOTE: Before using the drive, the kernel has to wait at least 420 nanoseconds to ensure
	/// that the drive is in a consistent state.
	fn select(&self) {
		let value = if !self.slave {
			SELECT_MASTER
		} else {
			SELECT_SLAVE
		};
		unsafe {
			io::outb(self.get_register_port(DRIVE_REGISTER_OFFSET), value);
		}
	}

	/// Waits at least 420 nanoseconds if `long` is not set, or at least 5 milliseconds if set.
	fn wait(&self, long: bool) {
		let port = self.get_register_port(STATUS_REGISTER_OFFSET);
		let count = if long {
			167
		} else {
			14
		};

		// Individual status read take at least 30ns. 30 * 14 = 420
		for _ in 0..count {
			unsafe {
				io::inb(port);
			}
		}
	}

	/// Flushes the drive's cache. The device is assumed to be selected.
	fn cache_flush(&self) {
		self.send_command(COMMAND_CACHE_FLUSH);
		self.wait_busy();
	}

	/// Sets the number `count` of sectors to read/write. The device is assumed to be selected.
	fn set_sectors_count(&self, count: u16) {
		unsafe {
			io::outw(SECTORS_COUNT_REGISTER_OFFSET, count);
		}
	}

	/// Sets the LBA offset `offset`. The device is assumed to be selected.
	fn set_lba(&self, offset: u64) {
		unsafe {
			io::outw(LBA_LO_REGISTER_OFFSET, ((offset >> 32) & 0xffff) as _);
			io::outw(LBA_MID_REGISTER_OFFSET, ((offset >> 16) & 0xffff) as _);
			io::outw(LBA_HI_REGISTER_OFFSET, (offset & 0xffff) as _);
		}
	}

	/// Resets both master and slave devices. The current drive may not be selected anymore.
	fn reset(&self) {
		let port = if !self.secondary {
			PRIMARY_DEVICE_CONTROL_PORT
		} else {
			SECONDARY_DEVICE_CONTROL_PORT
		};

		unsafe {
			io::outb(port, 1 << 2);
		}

		self.wait(true);

		unsafe {
			io::outb(port, 0);
		}

		self.wait(true);
	}

	/// Identifies the drive, retrieving informations about the drive. On error, the function
	/// returns a string telling the cause.
	fn identify(&mut self) -> Result<(), &'static str> {
		self.reset();
		self.select();
		self.wait(false);

		self.set_sectors_count(0);
		self.set_lba(0);
		self.wait(false);

		self.send_command(COMMAND_IDENTIFY);
		self.wait(false);

		let status = self.get_status();
		if status == 0 {
			return Err("Drive doesn't exist");
		}
		self.wait_busy();

		let lba_mid = unsafe {
			io::inb(self.get_register_port(LBA_MID_REGISTER_OFFSET))
		};
		let lba_hi = unsafe {
			io::inb(self.get_register_port(LBA_HI_REGISTER_OFFSET))
		};

		let atapi = lba_mid == 0x14 && lba_hi == 0xeb;
		let sata = lba_mid == 0x3c && lba_hi == 0xc3;
		if !atapi && !sata && (lba_mid != 0 || lba_hi != 0) {
			return Err("Unknown device");
		}

		if !atapi && !sata {
			loop {
				let status = self.get_status();
				if status & STATUS_DRQ != 0 || status & STATUS_ERR != 0 {
					break;
				}
			}

			if self.get_status() & STATUS_ERR != 0 {
				return Err("Error while identifying the device");
			}
		}

		let data_port = self.get_register_port(DATA_REGISTER_OFFSET);
		let mut data: [u16; 256] = [0; 256];
		for d in data.iter_mut() {
			*d = unsafe {
				io::inw(data_port)
			};
		}

		let lba48_support = data[83] & (1 << 10) != 0;
		let lba28_size = (data[60] as u32) | ((data[61] as u32) << 16);
		let lba48_size = (data[100] as u64) | ((data[101] as u64) << 16)
			| ((data[102] as u64) << 32) | ((data[103] as u64) << 48);

		if lba28_size == 0 {
			return Err("Unsupported disk (too old)");
		}

		self.lba48 = lba48_support;
		self.atapi = atapi;
		self.sata = sata;
		self.sectors_count = if lba48_support {
			lba48_size
		} else {
			lba28_size as _
		};

		self.wait(false);

		Ok(())
	}

	/// Waits for the drive to be ready for IO operation. The device is assumed to be selected.
	fn wait_io(&self) -> Result<(), Errno> {
		loop {
			let status = self.get_status();
			if (status & STATUS_BSY == 0) && (status & STATUS_DRQ != 0) {
				return Ok(());
			}
			if (status & STATUS_ERR != 0) || (status & STATUS_DF != 0) {
				return Err(errno::EIO);
			}
		}
	}

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	/// The function uses LBA28, thus the offset is assumed to be in range.
	fn read28(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		self.select();
		self.wait(true);

		let blocks_count = math::ceil_division(size, 256) as usize;
		for i in 0..blocks_count {
			unsafe {
				let drive = if self.slave {
					0xf0
				} else {
					0xe0
				} | ((offset >> 24) & 0x0f) as u8;
				let count = (size % 256) as u8;
				let lo_lba = (offset & 0xff) as u8;
				let mid_lba = ((offset >> 8) & 0xff) as u8;
				let hi_lba = ((offset >> 16) & 0xff) as u8;

				io::outb(self.get_register_port(DRIVE_REGISTER_OFFSET), drive);
				io::outb(self.get_register_port(SECTORS_COUNT_REGISTER_OFFSET), count);
				io::outb(self.get_register_port(LBA_LO_REGISTER_OFFSET), lo_lba);
				io::outb(self.get_register_port(LBA_MID_REGISTER_OFFSET), mid_lba);
				io::outb(self.get_register_port(LBA_HI_REGISTER_OFFSET), hi_lba);
			}

			self.send_command(COMMAND_READ_SECTORS);

			let data_port = self.get_register_port(DATA_REGISTER_OFFSET);
			for j in 0..(size as usize) {
				self.wait_io()?;

				for k in 0..256 {
					let index = ((i * 256 * 256) + (j * 256) + k) * 2;
					let word = unsafe {
						io::inw(data_port)
					};

					buf[index] = (word & 0xff) as _;
					buf[index + 1] = ((word >> 8) & 0xff) as _;
				}
			}
		}

		Ok(())
	}

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	/// The function uses LBA48.
	fn read48(&self, _buf: &mut [u8], _offset: u64, _size: u64) -> Result<(), Errno> {
		// TODO
		todo!();
	}

	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	/// The function uses LBA28, thus the offset is assumed to be in range.
	fn write28(&self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno> {
		self.select();
		self.wait(true);

		let blocks_count = math::ceil_division(size, 256) as usize;
		for i in 0..blocks_count {
			unsafe {
				let drive = if self.slave {
					0xf0
				} else {
					0xe0
				} | ((offset >> 24) & 0x0f) as u8;
				let count = (size % 256) as u8;
				let lo_lba = (offset & 0xff) as u8;
				let mid_lba = ((offset >> 8) & 0xff) as u8;
				let hi_lba = ((offset >> 16) & 0xff) as u8;

				io::outb(self.get_register_port(DRIVE_REGISTER_OFFSET), drive);
				io::outb(self.get_register_port(SECTORS_COUNT_REGISTER_OFFSET), count);
				io::outb(self.get_register_port(LBA_LO_REGISTER_OFFSET), lo_lba);
				io::outb(self.get_register_port(LBA_MID_REGISTER_OFFSET), mid_lba);
				io::outb(self.get_register_port(LBA_HI_REGISTER_OFFSET), hi_lba);
			}

			self.send_command(COMMAND_WRITE_SECTORS);

			let data_port = self.get_register_port(DATA_REGISTER_OFFSET);
			for j in 0..(size as usize) {
				self.wait_io()?;

				for k in 0..256 {
					let index = ((i * 256 * 256) + (j * 256) + k) * 2;
					let word = ((buf[index + 1] as u16) << 8) | (buf[index] as u16);

					unsafe {
						io::outw(data_port, word)
					}
				}
			}

			self.cache_flush();
		}

		Ok(())
	}

	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	/// The function uses LBA48.
	fn write48(&self, _buf: &[u8], _offset: u64, _size: u64) -> Result<(), Errno> {
		// TODO
		todo!();
	}
}

impl StorageInterface for PATAInterface {
	fn get_block_size(&self) -> u64 {
		512
	}

	fn get_blocks_count(&self) -> u64 {
		self.sectors_count
	}

	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		if offset >= self.sectors_count || offset + size >= self.sectors_count {
			return Err(errno::EINVAL);
		}

		if offset < (1 << 29) - 1 {
			self.read28(buf, offset, size)
		} else {
			self.read48(buf, offset, size)
		}
	}

	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno> {
		if offset >= self.sectors_count || offset + size >= self.sectors_count {
			return Err(errno::EINVAL);
		}

		if offset < (1 << 29) - 1 {
			self.write28(buf, offset, size)
		} else {
			self.write48(buf, offset, size)
		}
	}
}
