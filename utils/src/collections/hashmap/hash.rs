/*
 * Copyright 2015 The Rust Project Developers. See the COPYRIGHT
 * file at the top-level directory of this distribution and at
 * http://rust-lang.org/COPYRIGHT.
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

//! Implementation of the fxhash algorithm.
//!
//! fxhash is a non-cryptographic hash function meant to be used in hash maps. Also, the algorithm
//! does not provide protection against the HashDOS attack.
//!
//! Most of this implementation comes from [this repository](https://github.com/cbreeden/fxhash).
//! It has been modified to not require external dependencies.

#![allow(dead_code)]

use core::{hash::Hasher, ops::BitXor};

const ROTATE: u32 = 5;
const SEED64: u64 = 0x51_7c_c1_b7_27_22_0a_95;
const SEED32: u32 = 0x9e_37_79_b9;

#[cfg(target_pointer_width = "32")]
const SEED: usize = SEED32 as usize;
#[cfg(target_pointer_width = "64")]
const SEED: usize = SEED64 as usize;

trait HashWord {
	fn hash_word(&mut self, word: Self);
}

macro_rules! impl_hash_word {
    ($($ty:ty = $key:ident),* $(,)*) => (
        $(
            impl HashWord for $ty {
                #[inline]
                fn hash_word(&mut self, word: Self) {
                    *self = self.rotate_left(ROTATE).bitxor(word).wrapping_mul($key);
                }
            }
        )*
    )
}

impl_hash_word!(usize = SEED, u32 = SEED32, u64 = SEED64);

#[inline]
fn write32(mut hash: u32, mut bytes: &[u8]) -> u32 {
	while bytes.len() >= 4 {
		hash.hash_word(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
		bytes = &bytes[4..];
	}
	if bytes.len() >= 2 {
		hash.hash_word(u16::from_ne_bytes([bytes[0], bytes[1]]) as u32);
		bytes = &bytes[2..];
	}
	if let Some(&byte) = bytes.first() {
		hash.hash_word(u32::from(byte));
	}
	hash
}

#[inline]
fn write64(mut hash: u64, mut bytes: &[u8]) -> u64 {
	while bytes.len() >= 8 {
		hash.hash_word(u64::from_ne_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
		]));
		bytes = &bytes[8..];
	}
	if bytes.len() >= 4 {
		hash.hash_word(u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as _);
		bytes = &bytes[4..];
	}
	if bytes.len() >= 2 {
		hash.hash_word(u16::from_ne_bytes([bytes[0], bytes[1]]) as u64);
		bytes = &bytes[2..];
	}
	if let Some(&byte) = bytes.first() {
		hash.hash_word(u64::from(byte));
	}
	hash
}

#[inline]
#[cfg(target_pointer_width = "32")]
fn write(hash: usize, bytes: &[u8]) -> usize {
	write32(hash as u32, bytes) as usize
}

#[inline]
#[cfg(target_pointer_width = "64")]
fn write(hash: usize, bytes: &[u8]) -> usize {
	write64(hash as u64, bytes) as usize
}

/// Hasher with the fxhash algorithm.
#[derive(Default)]
pub struct FxHasher(usize);

impl Hasher for FxHasher {
	#[inline]
	fn write(&mut self, bytes: &[u8]) {
		self.0 = write(self.0, bytes);
	}

	#[inline]
	fn write_u8(&mut self, i: u8) {
		self.0.hash_word(i as usize);
	}

	#[inline]
	fn write_u16(&mut self, i: u16) {
		self.0.hash_word(i as usize);
	}

	#[inline]
	fn write_u32(&mut self, i: u32) {
		self.0.hash_word(i as usize);
	}

	#[inline]
	#[cfg(target_pointer_width = "32")]
	fn write_u64(&mut self, i: u64) {
		self.0.hash_word(i as usize);
		self.0.hash_word((i >> 32) as usize);
	}

	#[inline]
	#[cfg(target_pointer_width = "64")]
	fn write_u64(&mut self, i: u64) {
		self.0.hash_word(i as usize);
	}

	#[inline]
	fn write_usize(&mut self, i: usize) {
		self.0.hash_word(i);
	}

	#[inline]
	fn finish(&self) -> u64 {
		self.0 as u64
	}
}
