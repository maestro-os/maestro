/// This module implements the PATA interface for hard drives.
/// The PATA interface is an old, deprecated interface that has been replaced by the SATA
/// interface.
/// ATA devices may be detected by the PCI, but if not, it doesn't mean that they are not present.
/// The disk(s) may instead use the standardized IO ports for legacy support.

use super::StorageInterface;

// TODO Check ports

/// The beginning of the port range for the primary ATA bus.
const PRIMARY_ATA_BUS_PORT_BEGIN: u16 = 0x1f0;
/// The end of the port range for the primary ATA bus.
const PRIMARY_ATA_BUS_PORT_END: u16 = 0x1f7;

/// The port for the primary disk's device control register.
const PRIMARY_DEVICE_CONTROL_PORT: u16 = 0x3f6;
/// The port for the primary disk's alternate status register.
const PRIMARY_ALTERNATE_STATUS_PORT: u16 = 0x3f6;

/// The beginning of the port range for the secondary ATA bus.
const SECONDARY_ATA_BUS_PORT_BEGIN: u16 = 0x170;
/// The end of the port range for the secondary ATA bus.
const SECONDARY_ATA_BUS_PORT_END: u16 = 0x177;

/// The port for the secondary disk's device control register.
const SECONDARY_DEVICE_CONTROL_PORT: u16 = 0x3e6;
/// The port for the secondary disk's alternate status register.
const SECONDARY_ALTERNATE_STATUS_PORT: u16 = 0x3e6;

/// Flush cache command.
const ATA_COMMAND_CACHE_FLUSH: u8 = 0xe7;

/// Address mark not found.
const ATA_ERROR_AMNF: u8  = 0b00000001;
/// Track zero not found.
const ATA_ERROR_TKZNF: u8 = 0b00000010;
/// Aborted command.
const ATA_ERROR_ABRT: u8  = 0b00000100;
/// Media change request.
const ATA_ERROR_MCR: u8   = 0b00001000;
/// ID not found.
const ATA_ERROR_IDNF: u8  = 0b00010000;
/// Media changed.
const ATA_ERROR_MC: u8    = 0b00100000;
/// Uncorrectable data error.
const ATA_ERROR_UNC: u8   = 0b01000000;
/// Bad block detected.
const ATA_ERROR_BBK: u8   = 0b10000000;

/// Indicates an error occurred.
const ATA_STATUS_ERR: u8 = 0b00000001;
/// Set when drive has PIO data to transfer or is ready to accept PIO data.
const ATA_STATUS_DRQ: u8 = 0b00001000;
/// Overlapped Mode Service Request.
const ATA_STATUS_SRV: u8 = 0b00010000;
/// Drive Fault Error.
const ATA_STATUS_DF: u8 = 0b00100000;
/// Clear after an error. Set otherwise.
const ATA_STATUS_RDY: u8 = 0b01000000;
/// Indicates the drive is preparing to send/receive data.
const ATA_STATUS_BSY: u8 = 0b10000000;

/// Structure representing a PATA interface. An instance is associated with a unique disk.
pub struct PATAInterface {
	/// Tells whether the disk is slave or master.
	slave: bool,
}

impl PATAInterface {
	/// Creates a new instance.
	pub fn new(slave: bool) -> Self {
		Self {
			slave: slave,
		}
	}

	/// Tells whether the interface is for the slave disk.
	pub fn is_slave(&self) -> bool {
		self.slave
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
		// TODO
		0
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
