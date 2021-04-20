/// This module implements the PATA interface for hard drives.
/// The PATA interface is an old, deprecated interface that has been replaced by the SATA
/// interface.

use super::StorageInterface;

/// Structure representing a PATA interface. An instance is associated with a unique disk.
pub struct PATAInterface {
	// TODO disk informations
}

impl PATAInterface {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			// TODO
		}
	}
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
