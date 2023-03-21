//! This module implements randomness functions.

use crate::crypto::chacha20;
use crate::errno::Errno;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::lock::IntMutex;

/// The size of the entropy buffer in bytes.
const ENTROPY_BUFFER_SIZE: usize = 32768;
/// The minimum number of bytes needed to read entropy.
const ENTROPY_THRESHOLD: usize = 1024;

// TODO Implement entropy extraction (Fast Key Erasure?)

/// An entropy pool.
pub struct EntropyPool {
	/// Data pending to be treated. This buffer is used as a cache when input data is not large
	/// enough.
	pending: RingBuffer<u8, Vec<u8>>,
	/// The buffer containing entropy.
	buff: RingBuffer<u8, Vec<u8>>,

	/// The ChaCha20 counter.
	counter: u64,

	/// The seed to be used for pseudo-random generation (when the pool runs out of entropy).
	pseudo_seed: u64,
}

impl EntropyPool {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			pending: RingBuffer::new(crate::vec![0; 56]?),
			buff: RingBuffer::new(crate::vec![0; ENTROPY_BUFFER_SIZE]?),

			counter: 0,

			pseudo_seed: 0,
		})
	}

	/// Returns the number of available bytes.
	pub fn available_bytes(&self) -> usize {
		self.buff.get_data_len()
	}

	/// Reads entropy from the pool.
	///
	/// The function returns the number of bytes read.
	///
	/// If the pool do not contain enough entropy, the function returns `0`, unless
	/// `bypass_threshold` is set to `true`. In which case, randomness is not guaranteed.
	pub fn read(&mut self, buff: &mut [u8], bypass_threshold: bool) -> usize {
		let available = self.buff.get_data_len();

		if available < ENTROPY_THRESHOLD {
			if !bypass_threshold {
				return 0;
			}

			let mut seed = self.pseudo_seed;
			for b in buff.iter_mut() {
				seed = 6364136223846793005u64.wrapping_mul(seed).wrapping_add(1);
				*b = (seed & 0xff) as _;
			}
			self.pseudo_seed = seed;

			buff.len()
		} else {
			self.buff.read(buff)
		}
	}

	/// Upddates the seed for the pseudorandom generator.
	fn update_seed(&mut self, buff: &[u8; 8]) {
		let mut seed = 0;
		for (i, b) in buff.iter().enumerate() {
			seed |= (*b as u64) << (i * 8);
		}

		self.pseudo_seed = seed;
	}

	/// Writes entropy to the pool.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buff: &[u8]) -> usize {
		let mut off = 0;
		let mut total = 0;

		let mut input: [u8; 64] = [0; 64];
		let mut output: [u8; 64] = [0; 64];

		// Encode with ChaCha20
		while off < buff.len() && buff.len() - off >= self.pending.get_size() {
			// Add data
			input[0..48].copy_from_slice(&buff[off..(off + 48)]);
			off += 48;

			// Add counter to buffer
			for i in 0..8 {
				input[48 + i] = ((self.counter >> (i * 8)) & 0xff) as _;
			}
			off += 8;

			// Add nonce
			input[56..].copy_from_slice(&buff[off..(off + 8)]);
			off += 8;

			// Encode
			chacha20::block(&input, &mut output);

			// Update pseudo seed
			let mut seed_in: [u8; 8] = [0; 8];
			seed_in.copy_from_slice(&input[0..8]);
			self.update_seed(&seed_in);

			let l = self.buff.write(&output);
			if l == 0 {
				break;
			}

			total += l;
			self.counter += 1;
		}

		// Put remaining bytes into pending buffer
		while off < buff.len() {
			let l = self.pending.write(&buff[off..]);
			if l == 0 {
				break;
			}

			total += l;
		}

		total
	}
}

/// The entropy pool.
pub static ENTROPY_POOL: IntMutex<Option<EntropyPool>> = IntMutex::new(None);

/// Initializes randomness sources.
pub fn init() -> Result<(), Errno> {
	*ENTROPY_POOL.lock() = Some(EntropyPool::new()?);

	Ok(())
}
