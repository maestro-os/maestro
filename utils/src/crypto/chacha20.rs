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
		#[allow(clippy::manual_rotate)]
		{
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
		}
	};
}

/// Computes a ChaCha20 block.
pub fn block(inout: &mut [u8; 64]) {
	let mut buf: [u32; 16] = [0; 16];
	unsafe {
		ptr::copy_nonoverlapping(inout.as_ptr(), buf.as_mut_ptr() as *mut u8, 64);
	}
	for _ in (0..20).step_by(2) {
		// Odd round
		quarter_round!(buf[0], buf[4], buf[8], buf[12]);
		quarter_round!(buf[1], buf[5], buf[9], buf[13]);
		quarter_round!(buf[2], buf[6], buf[10], buf[14]);
		quarter_round!(buf[3], buf[7], buf[11], buf[15]);
		// Even round
		quarter_round!(buf[0], buf[5], buf[10], buf[15]);
		quarter_round!(buf[1], buf[6], buf[11], buf[12]);
		quarter_round!(buf[2], buf[7], buf[8], buf[13]);
		quarter_round!(buf[3], buf[4], buf[9], buf[14]);
	}
	unsafe {
		ptr::copy_nonoverlapping(buf.as_ptr() as *mut u8, inout.as_mut_ptr(), 64);
	}
}

// TODO unit tests
