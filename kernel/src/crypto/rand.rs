/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements randomness functions.

use crate::crypto::chacha20;
use utils::{
	collections::{ring_buffer::RingBuffer, vec::Vec},
	errno::AllocResult,
	lock::IntMutex,
	vec,
};

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
	pub fn new() -> AllocResult<Self> {
		Ok(Self {
			pending: RingBuffer::new(vec![0; 56]?),
			buff: RingBuffer::new(vec![0; ENTROPY_BUFFER_SIZE]?),

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
			// TODO first, encode some bytes from `pending`, then use them (if available)
			if !bypass_threshold {
				return 0;
			}
			// Use a PRNG to create fake entropy
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

	/// Encodes data to fill the entropy buffer.
	///
	/// The function returns the number of consumed bytes from the given buffer.
	fn encode(&mut self, buff: &[u8]) -> usize {
		let mut off = 0;
		let mut encode_buff: [u8; 64] = [0; 64];
		while off < buff.len() && buff.len() - off >= self.pending.get_size() {
			// Add data
			encode_buff[0..48].copy_from_slice(&buff[off..(off + 48)]);
			off += 48;
			// Add counter to buffer
			encode_buff[48..56].copy_from_slice(&self.counter.to_ne_bytes());
			// Add nonce
			encode_buff[56..].copy_from_slice(&buff[off..(off + 8)]);
			off += 8;

			// Encode with ChaCha20
			chacha20::block(&mut encode_buff);
			// Update pseudo seed
			let mut seed: [u8; 8] = [0; 8];
			seed.copy_from_slice(&encode_buff[..8]);
			self.pseudo_seed = u64::from_ne_bytes(seed);

			// Write
			let l = self.buff.write(&encode_buff[8..]);
			if l == 0 {
				break;
			}
			self.counter += 1;
		}
		off
	}

	/// Writes entropy to the pool.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buff: &[u8]) -> usize {
		let mut off = 0;
		if !self.buff.is_full() {
			off = self.encode(buff);
		}
		// Put remaining bytes into pending buffer
		while off < buff.len() {
			let l = self.pending.write(&buff[off..]);
			if l == 0 {
				break;
			}
			off += l;
		}
		buff.len() - off
	}
}

/// The entropy pool.
pub static ENTROPY_POOL: IntMutex<Option<EntropyPool>> = IntMutex::new(None);

/// Initializes randomness sources.
pub(super) fn init() -> AllocResult<()> {
	*ENTROPY_POOL.lock() = Some(EntropyPool::new()?);
	Ok(())
}
