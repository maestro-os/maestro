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

//! This module implements checksum algorithms. A checksum is a value allowing
//! to verify the integrity of a structure.

/// Computes a checksum on `data` according to RFC1071.
pub fn compute_rfc1071(data: &[u8]) -> u16 {
	let mut sum: u32 = 0;
	let mut i = 0;

	// Main loop
	while i < (data.len() & !1) {
		sum += ((data[i + 1] as u32) << 8) | (data[i] as u32);
		i += 2;
	}

	// Add remaining byte
	if i < data.len() {
		sum += data[i] as u32;
	}

	// Folding 32-bits value into 16-bits
	while (sum >> 16) != 0 {
		sum = (sum & 0xffff) + (sum >> 16);
	}

	(!sum) as u16
}

/// Computes the lookup table for the given generator polynomial.
///
/// Arguments:
/// - `table` is filled with the table's values.
/// - `polynom` is the polynom.
pub fn compute_crc32_lookuptable(table: &mut [u32; 256], polynom: u32) {
	// Little endian
	let mut i = table.len() / 2;
	let mut crc = 1;

	while i > 0 {
		if crc & 1 != 0 {
			crc = (crc >> 1) ^ polynom;
		} else {
			crc >>= 1;
		}

		for j in (0..table.len()).step_by(2 * i) {
			table[i ^ j] = crc ^ table[j];
		}

		i >>= 1;
	}
}

/// Computes the CRC32 checksum on the given data `data` with the given table
/// `table` for the wanted generator polynomial.
pub fn compute_crc32(data: &[u8], table: &[u32; 256]) -> u32 {
	// Sarwate algorithm
	let mut crc = !0u32;

	for b in data {
		let i = ((crc as usize) ^ (*b as usize)) & 0xff;
		crc = table[i] ^ (crc >> 8);
	}

	!crc
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn rfc1071_0() {
		for i in 0..=u16::MAX {
			let data = [(i & 0xff) as _, (i >> 8) as _];
			assert_eq!(compute_rfc1071(&data), !i);
		}
	}

	// TODO More tests on RFC1071
	// TODO Test CRC32
}
