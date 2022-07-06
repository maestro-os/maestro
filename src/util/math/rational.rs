//! A rational number is a number which can be represented as the fraction of two integers: `a / b`

use core::cmp::Ordering;
use core::cmp::PartialEq;
use core::ops::Add;
use core::ops::AddAssign;
use core::ops::Div;
use core::ops::DivAssign;
use core::ops::Mul;
use core::ops::MulAssign;
use core::ops::Neg;
use core::ops::Sub;
use core::ops::SubAssign;
use crate::util::math;

// FIXME: Operations can overflow

/// Structure implementing the representing a rational number.
#[derive(Copy, Clone, Debug)]
pub struct Rational {
	/// The numerator.
	a: i64,
	/// The denominator.
	b: i64,
}

impl Rational {
	/// Creates an instance from a given integer `n`.
	pub const fn from_integer(n: i64) -> Self {
		Self {
			a: n,
			b: 1,
		}
	}

	/// Returns the numerator of the number.
	pub fn get_numerator(&self) -> i64 {
		self.a
	}

	/// Returns the denominator of the number.
	pub fn get_denominator(&self) -> i64 {
		self.b
	}

	/// Converts the value to the nearest integer value.
	pub fn as_integer(&self) -> i64 {
		self.a / self.b
	}

	/// Reduces the fraction so that `a / b` becomes irreducible.
	pub fn reduce(&mut self) {
		let gcd = math::gcd(self.a, self.b);
		self.a /= gcd;
		self.b /= gcd;

		if self.b < 0 {
			self.a = -self.a;
			self.b = -self.b;
		}
	}
}

impl From<i64> for Rational {
	fn from(n: i64) -> Self {
		Self::from_integer(n)
	}
}

impl Neg for Rational {
	type Output = Self;

	fn neg(mut self) -> Self {
		self.a = -self.a;
		self
	}
}

impl Add for Rational {
	type Output = Self;

	fn add(self, other: Self) -> Self {
		let mut s = Self {
			a: (self.a * other.b) + (other.a * self.b),
			b: self.b * other.b,
		};
		s.reduce();
		s
	}
}

impl Add<i64> for Rational {
	type Output = Self;

	fn add(self, other: i64) -> Self {
		let mut s = Self {
			a: self.a + (other * self.b),
			b: self.b,
		};
		s.reduce();
		s
	}
}

impl Sub for Rational {
	type Output = Self;

	fn sub(self, other: Self) -> Self {
		let mut s = Self {
			a: (self.a * other.b) - (other.a * self.b),
			b: self.b * other.b,
		};
		s.reduce();
		s
	}
}

impl Sub<i64> for Rational {
	type Output = Self;

	fn sub(self, other: i64) -> Self {
		let mut s = Self {
			a: self.a - (other * self.b),
			b: self.b,
		};
		s.reduce();
		s
	}
}

impl Mul for Rational {
	type Output = Self;

	fn mul(self, other: Self) -> Self {
		let mut s = Self {
			a: self.a * other.a,
			b: self.b * other.b,
		};
		s.reduce();
		s
	}
}

impl Mul<i64> for Rational {
	type Output = Self;

	fn mul(self, other: i64) -> Self {
		let mut s = Self {
			a: self.a * other,
			b: self.b,
		};
		s.reduce();
		s
	}
}

// TODO Watch for division by 0
impl Div for Rational {
	type Output = Self;

	fn div(self, other: Self) -> Self {
		let mut s = Self {
			a: self.a * other.b,
			b: self.b * other.a,
		};
		s.reduce();
		s
	}
}

// TODO Watch for division by 0
impl Div<i64> for Rational {
	type Output = Self;

	fn div(self, other: i64) -> Self {
		let mut s = Self {
			a: self.a,
			b: self.b * other,
		};
		s.reduce();
		s
	}
}

impl AddAssign for Rational {
	fn add_assign(&mut self, other: Self) {
		*self = *self + other;
	}
}

impl SubAssign for Rational {
	fn sub_assign(&mut self, other: Self) {
		*self = *self - other;
	}
}

impl MulAssign for Rational {
	fn mul_assign(&mut self, other: Self) {
		*self = *self * other;
	}
}

impl DivAssign for Rational {
	fn div_assign(&mut self, other: Self) {
		*self = *self / other;
	}
}

impl Eq for Rational {}

impl PartialEq for Rational {
	fn eq(&self, other: &Self) -> bool {
		self.a == other.a && self.b == other.b
	}
}

impl PartialOrd for Rational {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some((self.a * other.b).cmp(&(other.a * self.b)))
	}
}

// TODO Add printing function

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn rational_add() {
		assert_eq!(Rational::from(0) + Rational::from(0), Rational::from(0));
		assert_eq!(Rational::from(1) + Rational::from(1), Rational::from(2));
		assert_eq!(Rational::from(1) + Rational::from(2), Rational::from(3));
		assert_eq!(Rational::from(1) + Rational::from(-1), Rational::from(0));

		assert_eq!(Rational::from(1) / 2 + Rational::from(1) / 2, Rational::from(1));
		assert_eq!(Rational::from(1) / 3 + Rational::from(2) / 3, Rational::from(1));
		assert_eq!(Rational::from(1) / 2 + Rational::from(1) / 3, Rational::from(5) / 6);
	}

	#[test_case]
	fn rational_sub() {
		assert_eq!(Rational::from(0) - Rational::from(0), Rational::from(0));
		assert_eq!(Rational::from(1) - Rational::from(1), Rational::from(0));
		assert_eq!(Rational::from(1) - Rational::from(2), Rational::from(-1));
		assert_eq!(Rational::from(1) - Rational::from(-1), Rational::from(2));

		assert_eq!(Rational::from(1) / 2 - Rational::from(1) / 2, Rational::from(0));
		assert_eq!(Rational::from(1) / 3 - Rational::from(2) / 3, Rational::from(-1) / 3);
		assert_eq!(Rational::from(1) / 2 - Rational::from(1) / 3, Rational::from(1) / 6);
	}

	#[test_case]
	fn rational_mul() {
		assert_eq!(Rational::from(0) * Rational::from(0), Rational::from(0));
		assert_eq!(Rational::from(1) * Rational::from(1), Rational::from(1));
		assert_eq!(Rational::from(1) * Rational::from(2), Rational::from(2));
		assert_eq!(Rational::from(1) * Rational::from(-1), Rational::from(-1));

		assert_eq!(Rational::from(1) / 2 * Rational::from(1) / 2, Rational::from(1) / 4);
		assert_eq!(Rational::from(1) / 3 * Rational::from(2) / 3, Rational::from(2) / 9);
		assert_eq!(Rational::from(1) / 2 * Rational::from(1) / 3, Rational::from(1) / 6);
	}

	#[test_case]
	fn rational_div() {
		assert_eq!(Rational::from(1) / Rational::from(1), Rational::from(1));
		assert_eq!(Rational::from(1) / Rational::from(2), Rational::from(1) / 2);
		assert_eq!(Rational::from(1) / Rational::from(-1), Rational::from(1) / -1);

		assert_eq!((Rational::from(1) / 2) / (Rational::from(1) / 2), Rational::from(1));
		assert_eq!((Rational::from(1) / 3) / (Rational::from(2) / 3), Rational::from(1) / 2);
		assert_eq!((Rational::from(1) / 2) / (Rational::from(1) / 3), Rational::from(3) / 2);
	}
}
