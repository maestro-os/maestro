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

//! The functions in this module implement utilities for integer mathematics.
//!
//! Since floating point numbers are slow, unprecise and may even disabled by
//! default, the kernel uses only integers.

pub mod rational;

use core::ops::{Add, Div, Mul, Neg, Rem, Shl, Sub};

/// Computes `pow(2, n)` where `n` is unsigned.
///
/// The behaviour is undefined for n < 0.
#[inline(always)]
pub fn pow2<T>(n: T) -> T
where
	T: From<u8> + Shl<Output = T>,
{
	T::from(1) << n
}

/// Computes a linear interpolation over integers.
///
/// FIXME: doc is unclear
#[inline(always)]
pub fn integer_linear_interpolation<T>(x: T, a_x: T, a_y: T, b_x: T, b_y: T) -> T
where
	T: Copy
		+ Add<Output = T>
		+ Sub<Output = T>
		+ Mul<Output = T>
		+ Div<Output = T>
		+ Neg<Output = T>,
{
	a_y + ((x - a_x) * (-a_y + b_y)) / (b_x - a_x)
}

/// Pseudo random number generation based on linear congruential generator.
///
/// Arguments:
/// - `x` is the value to compute the next number from.
/// It should either be a seed, or the previous value returned from this function.
/// - `a`, `c` and `m` are hyperparameters use as follows: (a * x + c) % m.
pub fn pseudo_rand(x: u32, a: u32, c: u32, m: u32) -> u32 {
	a.wrapping_mul(x).wrapping_add(c) % m
}

/// Returns the Greatest Common Divider of the two given numbers.
pub fn gcd<T>(mut a: T, mut b: T) -> T
where
	T: Clone + From<u8> + PartialEq + Rem<Output = T>,
{
	while b != T::from(0) {
		let r = a % b.clone();
		a = b;
		b = r;
	}

	a
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn gcd0() {
		assert_eq!(gcd(2, 2), 2);
		assert_eq!(gcd(4, 2), 2);
		assert_eq!(gcd(4, 4), 4);
		assert_eq!(gcd(8, 12), 4);
		assert_eq!(gcd(48, 18), 6);
	}
}
