//! This module implements randomness functions.

use crate::crypto::chacha20;
use crate::memory::malloc;

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

/// Consomes entropy from the buffer to fill the given `buf`.
/// If the size of `buf` exceeds the size of the entropy buffer, the function returns None.
pub fn consume_entropy(_buf: &mut [u8]) -> Option<()> {
	// TODO
	todo!();
}

/// Trait representing a PseudoRandom Number Generator.
pub trait PRNG {
	/// Fills the given buffer `buf` with random bytes.
	/// If not enough entropy is available at the moment, the function may return None.
	fn rand(buf: &mut [u8]) -> Option<()>;
}

/// Random bytes generator using ChaCha20.
struct ChaCha20Rand {}

impl PRNG for ChaCha20Rand {
	fn rand(buf: &mut [u8]) -> Option<()> {
		let mut b = unsafe { // Safe because the buffer is filled after
			malloc::Alloc::<u8>::new_zero(buf.len()).ok()?
		};
		consume_entropy(b.get_slice_mut())?;

		let key: [u32; 8] = [0; 8]; // TODO Fill with entropy buffer
		let nonces: [u32; 3] = [0; 3]; // TODO Fill with entropy buffer
		// TODO On fail, feed the entropy back to the buffer to avoid waste

		chacha20::encode(b.get_slice(), &key, &nonces, buf);
		Some(())
	}
}

/// Fills the given buffer `buf` with random bytes using from the preferred source.
/// If not enough entropy is available at the moment, the function returns None.
pub fn rand(_buf: &mut [u8]) -> Option<()> {
	// TODO
	None
}
