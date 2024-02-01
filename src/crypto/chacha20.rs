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

//! Implementation of the ChaCha20 algorithm.

use core::ptr;

/// Performs a left rotation of `b` bits on the value `a`.
macro_rules! rotl {
	($a:expr, $b:expr) => {
		($a << $b) | ($a >> (32 - $b))
	};
}

/// Performs a quarter round on the given values.
macro_rules! quarter_round {
	($a:expr, $b:expr, $c:expr, $d:expr) => {
		$a = $a.wrapping_add($b);
		$d ^= $a;
		$d = rotl!($d, 16);

		$c = $c.wrapping_add($d);
		$b ^= $c;
		$b = rotl!($b, 12);

		$a = $a.wrapping_add($b);
		$d ^= $a;
		$d = rotl!($d, 8);

		$c = $c.wrapping_add($d);
		$b ^= $c;
		$b = rotl!($b, 7);
	};
}

/// Computes a ChaCha20 block.
pub fn block(inout: &mut [u8; 64]) {
	let mut buff: [u32; 16] = [0; 16];

	unsafe {
		ptr::copy_nonoverlapping(inout.as_ptr(), buff.as_mut_ptr() as *mut u8, 64);
	}

	for _ in (0..20).step_by(2) {
		// Odd round
		quarter_round!(buff[0], buff[4], buff[8], buff[12]);
		quarter_round!(buff[1], buff[5], buff[9], buff[13]);
		quarter_round!(buff[2], buff[6], buff[10], buff[14]);
		quarter_round!(buff[3], buff[7], buff[11], buff[15]);

		// Even round
		quarter_round!(buff[0], buff[5], buff[10], buff[15]);
		quarter_round!(buff[1], buff[6], buff[11], buff[12]);
		quarter_round!(buff[2], buff[7], buff[8], buff[13]);
		quarter_round!(buff[3], buff[4], buff[9], buff[14]);
	}

	unsafe {
		ptr::copy_nonoverlapping(buff.as_ptr() as *mut u8, inout.as_mut_ptr(), 64);
	}
}

// TODO unit tests
