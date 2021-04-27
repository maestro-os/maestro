/// This module implements storage drivers.

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

// TODO Take into account hotplug devices and buses (PCI, USB, ...)
// TODO Function to add a device

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
