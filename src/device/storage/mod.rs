/// This module implements storage drivers.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::storage::pata::PATAInterface;
use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;

pub mod pata;

/// Trait representing a storage interface. A storage block is the atomic unit for I/O access on
/// the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	fn get_block_size(&self) -> usize;
	/// Returns the alignment of the storage blocks in bytes.
	fn get_block_alignment(&self) -> usize;
	/// Returns the number of storage blocks.
	fn get_blocks_count(&self) -> u64;

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), ()>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	fn write(&self, buf: &[u8], offset: u64, size: u64) -> Result<(), ()>;
}

// TODO Function to add a device

/// Structure managing storage devices.
pub struct StorageManager {
	/// The list of detected interfaces.
	interfaces: Vec<Box<dyn StorageInterface>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			interfaces: Vec::new(),
		}
	}

	/// Adds a storage device.
	fn add(&mut self, storage: Box<dyn StorageInterface>) -> Result<(), Errno> {
		// TODO Read to detect partitions

		self.interfaces.push(storage)
	}
}

impl DeviceManager for StorageManager {
	fn legacy_detect(&mut self) -> Result<(), Errno> {
		// TODO Detect floppy disks

		for i in 0..4 {
			let secondary = (i & 0b01) != 0;
			let slave = (i & 0b10) != 0;

			if let Ok(dev) = PATAInterface::new(secondary, slave) {
				self.add(Box::new(dev)?)?;
			}
		}

		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}

/// Tests every storage drivers on every storage devices.
/// The execution of this function removes all the data on every connected writable disks, so it
/// must be used carefully.
#[cfg(config_debug_storagetest)]
pub fn test() {
	// TODO Iterate on every devices:
	// - Run several times:
	//  - Write pseudo-random indicator data on every blocks
	//  - Read to check indicators to make sure that I/O works
}
