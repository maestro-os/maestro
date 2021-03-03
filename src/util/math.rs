/// This module contains mathematical utility functions.

use crate::util;

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
pub fn pow2<T>(n: T) -> T
	where T: From<u8>
		+ core::ops::Shl<Output = T> {
	T::from(1) << n
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

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn log2_0() {
		debug_assert!(log2(0) == 0);
		//debug_assert!(log2(-1) == 0);
	}

	#[test_case]
	fn log2_1() {
		for i in 1..util::bit_size_of::<usize>() {
			debug_assert!(log2(pow2(i)) == i);
		}
	}

	// TODO Test every functions
}
