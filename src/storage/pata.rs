/// This module implements the PATA interface for hard drives.
/// The PATA interface is an old, deprecated interface that has been replaced by the SATA
/// interface.
/// ATA devices may be detected by the PCI, but if not, it doesn't mean that they are not present.
/// The disk(s) may instead use the standardized IO ports for legacy support.
///
/// Legacy PATA can support up to two buses, each supporting up to two drives.
/// Each bus has its own set of ports. Before using a drive, the kernel has to:
/// - Select the drive (with the dedicated command)
/// - Identify it to retrieve informations, such as whether the drives support LBA48
///
/// TODO

use core::cmp::min;
use crate::io;
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
const SECONDARY_DEVICE_CONTROL_PORT: u16 = 0x3e6; // TODO Check
/// The port for the secondary disk's alternate status register.
const SECONDARY_ALTERNATE_STATUS_PORT: u16 = 0x3e6; // TODO Check

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
const COMMAND_SELECT_MASTER: u8 = 0xa0;
/// Selects the slave drive.
const COMMAND_SELECT_SLAVE: u8 = 0xb0;
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

	/// The number of sectors on the disk.
	sectors_count: u64,
}

impl PATAInterface {
	/// Creates a new instance.
	/// `secondary` tells whether the disk is on the secondary bus.
	/// `slave` tells whether the disk is the slave disk.
	pub fn new(secondary: bool, slave: bool) -> Self {
		let mut s = Self {
			secondary: secondary,
			slave: slave,

			sectors_count: 0,
		};
		s.identify();
		s
	}

	/// Tells whether the interface is for the secondary bus.
	pub fn is_secondary(&self) -> bool {
		self.secondary
	}

	/// Tells whether the interface is for the slave disk.
	pub fn is_slave(&self) -> bool {
		self.slave
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
		if !self.slave {
			self.send_command(COMMAND_SELECT_MASTER);
		} else {
			self.send_command(COMMAND_SELECT_SLAVE);
		}
	}

	/// Waits at least 420 nanoseconds after a select operations.
	fn select_wait(&self) {
		let port = self.get_register_port(STATUS_REGISTER_OFFSET);

		// Individual status read take at least 30ns. 30 * 14 = 420
		for _ in 0..14 {
			unsafe {
				io::inb(port);
			}
		}
	}

	/// Identifies the drive, retrieving informations about the drive.
	fn identify(&mut self) {
		self.select();
		self.select_wait();

		self.send_command(COMMAND_IDENTIFY);
		// TODO Wait?

		let status = self.get_status();
		if status == 0 {
			// TODO Drive doesn't exist
			return;
		}
		self.wait_busy();

		let lba_mid = unsafe {
			io::inb(self.get_register_port(LBA_MID_REGISTER_OFFSET))
		};
		let lba_hi = unsafe {
			io::inb(self.get_register_port(LBA_HI_REGISTER_OFFSET))
		};
		if lba_mid != 0 || lba_hi != 0 {
			// TODO Not ATA
			return;
		}

		loop {
			let status = self.get_status();
			if status & STATUS_DRQ != 0 || status & STATUS_ERR != 0 {
				break;
			}
		}

		if self.get_status() & STATUS_ERR != 0 {
			// TODO Error while identifying
			return;
		}

		let data_port = self.get_register_port(DATA_REGISTER_OFFSET);
		let mut data: [u16; 256] = [0; 256];
		for i in 0..data.len() {
			data[i] = unsafe {
				io::inw(data_port)
			};
		}

		let lba48_support = data[83] & (1 << 10) != 0;
		let lba28_size = ((data[60] as u32) << 32) | (data[61] as u32);
		let lba48_size = ((data[100] as u64) << 48) | ((data[101] as u64) << 32)
			| ((data[102] as u64) << 16) | (data[103] as u64);

		if lba28_size == 0 {
			// TODO Unsupported disk
			return;
		}

		self.sectors_count = if lba48_support {
			lba48_size
		} else {
			lba28_size as _
		};
	}

	// TODO
}

impl StorageInterface for PATAInterface {
	fn get_block_size(&self) -> usize {
		512
	}

	fn get_block_alignment(&self) -> usize {
		512
	}

	fn get_blocks_count(&self) -> usize {
		min(self.sectors_count, usize::MAX as u64) as _
	}

	fn read(&self, _buf: &mut [u8], _offset: usize, _size: usize) -> Result<(), ()> {
		// TODO
		Err(())
	}

	fn write(&self, _buf: &[u8], _offset: usize, _size: usize) -> Result<(), ()> {
		// TODO
		Err(())
	}
}
