//! This module implements randomness functions.

/// The size of the entropy buffer in bytes.
const ENTROPY_BUFFER_SIZE: usize = 32768;

// /// The entropy buffer, storing input bytes for random number generators.
// TODO static ENTROPY_BUFFER: IMutex<> = IMutex::new();

/// Feeds data `data` to the random number generators.
/// If the entropy buffer is already full, the data is ignored.
pub fn feed_entropy(_data: &[u8]) {
	// TODO
	todo!();
}

/// Trait representing a PseudoRandom Number Generator.
pub trait PRNG {
	/// Fills the given buffer `buf` with random bytes.
	/// If not enough entropy is available at the moment, the function may return None.
	fn rand(buf: &mut [u8]) -> Option<()>;
}
