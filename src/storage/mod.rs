/// This module implements storage utilities.

pub mod pata;

/// Trait representing a storage interface. A storage block is the atomic unit for I/O access on
/// the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	fn get_block_size(&self) -> usize;
	/// Returns the alignment of the storage blocks in bytes.
	fn get_block_alignment(&self) -> usize;
	/// Returns the number of storage blocks.
	fn get_blocks_count(&self) -> usize;

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	fn read(&self, buf: &mut [u8], offset: usize, size: usize) -> Result<(), ()>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	fn write(&self, buf: &[u8], offset: usize, size: usize) -> Result<(), ()>;
}
