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

use crate::{
	crypto::chacha20,
	memory::{ring_buffer::RingBuffer, user::UserSlice},
	sync::mutex::IntMutex,
};
use core::{
	cmp::min,
	num::{NonZeroUsize, Wrapping},
};
use utils::errno::{AllocResult, EResult};

// TODO Implement entropy extraction (Fast Key Erasure?)

/// An entropy pool.
pub struct EntropyPool {
	/// Available, non-encoded entropy
	pending: RingBuffer,
	/// Unused remains of the last encoding round
	remain: RingBuffer,

	/// The ChaCha20 counter.
	counter: Wrapping<u64>,

	/// The seed to be used for pseudo-random generation (urandom).
	pseudo_seed: u64,
}

impl EntropyPool {
	/// Creates a new instance.
	pub fn new() -> AllocResult<Self> {
		Ok(Self {
			pending: RingBuffer::new(NonZeroUsize::new(32768).unwrap())?,
			remain: RingBuffer::new(NonZeroUsize::new(56).unwrap())?,

			counter: Wrapping::default(),

			pseudo_seed: 0,
		})
	}

	/// Reads data from the pending entropy buffer, encodes it and writes it in `dst`.
	///
	/// If not enough entropy is available, the function returns `false`
	fn encode(&mut self, dst: &mut [u8; 64]) -> EResult<bool> {
		// Read data from the pending entropy buffer
		let mut src = [0u8; 56];
		if self.pending.get_data_len() < src.len() {
			return Ok(false);
		}
		self.pending.read(UserSlice::from_slice_mut(&mut src))?;
		// Add data
		dst[0..48].copy_from_slice(&src[..48]);
		// Add counter to buffer
		dst[48..56].copy_from_slice(&self.counter.0.to_ne_bytes());
		// Add nonce
		dst[56..].copy_from_slice(&src[48..]);
		// Encode with ChaCha20
		chacha20::block(dst);
		// Update pseudo seed
		let mut seed: [u8; 8] = [0; 8];
		seed.copy_from_slice(&dst[..8]);
		self.pseudo_seed = u64::from_ne_bytes(seed);
		// Update counter
		self.counter += 1;
		Ok(true)
	}

	/// Reads entropy from the pool.
	///
	/// Arguments:
	/// - `buf` is where random bytes are written to
	/// - `random`: if `true`, limit randomness to the available entropy, returning just the amount
	///   that could be read
	/// - `nonblocking`: if `true`, do not block if entropy is missing
	///
	/// The function returns the number of bytes read.
	pub fn read(
		&mut self,
		buf: UserSlice<u8>,
		random: bool,
		_nonblocking: bool,
	) -> EResult<usize> {
		// First, use remaining used entropy
		let mut off = self.remain.read(buf)?;
		// If we need more entropy, iterate
		let mut encode_buf = [0u8; 64];
		while off < buf.len() {
			let res = self.encode(&mut encode_buf)?;
			// If not enough entropy is available
			if !res {
				// TODO if blocking, block until enough entropy is available
				if !random {
					// urandom is allowed: use a PRNG
					let mut seed = self.pseudo_seed;
					for b in encode_buf.iter_mut() {
						seed = 6364136223846793005u64.wrapping_mul(seed).wrapping_add(1);
						*b = (seed & 0xff) as _;
					}
					self.pseudo_seed = seed;
				} else {
					// urandom is not allowed, stop
					break;
				}
			}
			// Copy to user
			let l = min(buf.len() - off, encode_buf.len());
			buf.copy_to_user(off, &encode_buf[..l])?;
			// Keep remaining bytes
			self.remain
				.write(UserSlice::from_slice_mut(&mut encode_buf[l..]))?;
			off += l;
		}
		Ok(off)
	}

	/// Writes entropy to the pool.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buf: UserSlice<u8>) -> EResult<usize> {
		self.pending.write(buf)
	}
}

/// The entropy pool.
pub static ENTROPY_POOL: IntMutex<Option<EntropyPool>> = IntMutex::new(None);

/// Initializes randomness sources.
pub(super) fn init() -> AllocResult<()> {
	*ENTROPY_POOL.lock() = Some(EntropyPool::new()?);
	Ok(())
}
