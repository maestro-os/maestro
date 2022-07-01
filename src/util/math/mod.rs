//! Since floating point numbers are slow, unprecise and may even disabled by default, the kernel
/// uses! only integers. The functions in this module implement utilities for integer mathematics.

use core::intrinsics::wrapping_add;
use core::intrinsics::wrapping_mul;
use crate::util;

pub mod rational;

/// Clamps the given value `n` between `min` and `max`.
pub fn clamp<T: PartialOrd>(n: T, min: T, max: T) -> T {
	if n < min {
		min
	} else if n > max {
		max
	} else {
		n
	}
}

/// Computes ceil(n0 / n1) without using floating point numbers.
#[inline(always)]
pub fn ceil_division<T>(n0: T, n1: T) -> T
	where T: From<u8> + Copy
		+ core::ops::Add<Output = T>
		+ core::ops::Div<Output = T>
		+ core::ops::Rem<Output = T>
		+ core::cmp::PartialEq {
	if (n0 % n1) != T::from(0) {
		(n0 / n1) + T::from(1)
	} else {
		n0 / n1
	}
}

/// Computes 2^^n on unsigned integers (where `^^` is an exponent).
/// The behaviour is undefined for n < 0.
#[inline(always)]
pub fn pow2<T>(n: T) -> T where T: From<u8> + core::ops::Shl<Output = T> {
	T::from(1) << n
}

/// Computes a^^b on integers (where `^^` is an exponent).
#[inline(always)]
pub fn pow<T>(a: T, b: usize) -> T where T: From<u8> + core::ops::Mul<Output = T> + Copy {
	if b == 0 {
		T::from(1)
	} else if b == 1 {
		a
	} else if b % 2 == 0 {
		pow(a * a, b / 2)
	} else {
		a * pow(a * a, b / 2)
	}
}

/// Computes floor(log2(n)) on unsigned integers without using floating-point numbers.
/// Because the logarithm is undefined for n <= 0, the function returns `0` in this case.
#[inline(always)]
pub fn log2<T>(n: T) -> T
	where T: From<usize>
		+ Into<usize>
		+ core::cmp::PartialOrd
		+ core::ops::Sub<Output = T> {
	if n > T::from(0) {
		T::from(util::bit_size_of::<T>()) - T::from(n.into().leading_zeros() as _) - T::from(1)
	} else {
		T::from(0)
	}
}

/// Tells whether the given number is a power of two.
/// If `n` is zero, the behaviour is undefined.
pub fn is_power_of_two<T>(n: T) -> bool
	where T: Copy
		+ From<u8>
		+ core::ops::BitAnd<Output = T>
		+ core::ops::Sub<Output = T>
		+ core::cmp::PartialEq {
	n == T::from(0) || n & (n - T::from(1)) == T::from(0)
}

/// Computes a linear interpolation over integers.
/// The function computes the interpolation coefficient relative to the parameters `x`, `a_x` and
/// `b_x`.
#[inline(always)]
pub fn integer_linear_interpolation<T>(x: T, a_x: T, a_y: T, b_x: T, b_y: T) -> T
	where T: Copy
		+ core::ops::Add<Output = T>
		+ core::ops::Sub<Output = T>
		+ core::ops::Mul<Output = T>
		+ core::ops::Div<Output = T>
		+ core::ops::Neg<Output = T> {
	a_y + ((x - a_x) * (-a_y + b_y)) / (b_x - a_x)
}

/// Pseudo random number generation based on linear congruential generator.
/// `x` is the value to compute the next number from. It should either be a seed, or the previous
/// value returned from this function.
/// `a`, `c` and `m` are hyperparameters use as follows: (a * x + c) % m.
pub fn pseudo_rand(x: u32, a: u32, c: u32, m: u32) -> u32 {
	(wrapping_add(wrapping_mul(a, x), c)) % m
}

/// Returns the Greatest Common Divider of the two given numbers.
pub fn gcd<T>(mut a: T, mut b: T) -> T
	where T: Clone
		+ From<u8>
		+ core::cmp::PartialEq
		+ core::ops::Rem<Output = T> {
	while b != T::from(0) {
		let r = a % b.clone();
		a = b;
		b = r;
	}

	return a;
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn log2_0() {
		assert_eq!(log2(0), 0);
		//assert_eq!(log2(-1), 0);
	}

	#[test_case]
	fn log2_1() {
		for i in 1..util::bit_size_of::<usize>() {
			assert_eq!(log2(pow2(i)), i);
		}
	}

	#[test_case]
	fn pow0() {
		for i in 0..10 {
			assert_eq!(pow::<u32>(1, i), 1);
		}
	}

	#[test_case]
	fn pow1() {
		for i in 0..10 {
			assert_eq!(pow::<u32>(2, i), pow2::<u32>(i as _));
		}
	}

	#[test_case]
	fn pow2() {
		assert_eq!(pow::<u32>(10, 0), 1);
		assert_eq!(pow::<u32>(10, 1), 10);
		assert_eq!(pow::<u32>(10, 2), 100);
		assert_eq!(pow::<u32>(10, 3), 1000);
		assert_eq!(pow::<u32>(10, 4), 10000);
		assert_eq!(pow::<u32>(10, 5), 100000);
	}

	#[test_case]
	fn gcd() {
		assert_eq!(gcd(2, 2), 2);
		assert_eq!(gcd(4, 2), 2);
		assert_eq!(gcd(4, 4), 4);
		assert_eq!(gcd(8, 12), 4);
		assert_eq!(gcd(48, 18), 6);
	}

	#[test_case]
	fn is_power_of_two() {
		for i in 0..31 {
			let n = (1 as u32) << i;
			debug_assert!(is_power_of_two(n));
			debug_assert!(!is_power_of_two(!n));
		}
	}
}
