//! This module implements types representing timestamps.

use core::cmp::Ordering;
use core::ops::Add;

/// Type representing a timestamp in seconds. Equivalent to POSIX's `time_t`.
pub type Timestamp = u64;
/// Type representing a timestamp in microseconds. Equivalent to POSIX's `suseconds_t`.
pub type UTimestamp = u64;
/// Type representing an elapsed number of ticks. Equivalent to POSIX's `clock_t`.
pub type Clock = u32;

/// Trait to be implement on a structure describing a moment in time.
pub trait TimeUnit: Sized + Clone + Default + Add<Self, Output = Self> + PartialOrd {
	/// Creates the structure from the given timestamp in nanoseconds.
	fn from_nano(timestamp: u64) -> Self;
	/// Returns the equivalent timestamp in nanoseconds.
	fn to_nano(&self) -> u64;

	/// Tells whether the corresponding timestamp is zero.
	fn is_zero(&self) -> bool {
		self.to_nano() == 0
	}
}

/// POSIX structure representing a timestamp.
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Timeval {
	/// Seconds
	pub tv_sec: Timestamp,
	/// Microseconds
	pub tv_usec: UTimestamp,
}

impl TimeUnit for Timeval {
	fn from_nano(timestamp: u64) -> Self {
		let sec = timestamp / 1000000000;
		let usec = (timestamp % 1000000000) / 1000;

		Self {
			tv_sec: sec,
			tv_usec: usec,
		}
	}

	fn to_nano(&self) -> u64 {
		self.tv_sec * 1000000000 + self.tv_usec * 1000
	}
}

impl Add<Timeval> for Timeval {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec + rhs.tv_sec,
			tv_usec: self.tv_usec + rhs.tv_usec,
		}
	}
}

impl PartialEq for Timeval {
	fn eq(&self, other: &Self) -> bool {
		self.tv_sec == other.tv_sec && self.tv_usec == other.tv_usec
	}
}

impl PartialOrd for Timeval {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.tv_sec.cmp(&other.tv_sec).then_with(|| self.tv_usec.cmp(&other.tv_usec)))
	}
}

/// Same as `Timeval`, but with nanosecond precision.
#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct Timespec {
	/// Seconds
	pub tv_sec: Timestamp,
	/// Nanoseconds
	pub tv_nsec: u32,
}

impl TimeUnit for Timespec {
	fn from_nano(timestamp: u64) -> Self {
		let sec = timestamp / 1000000000;
		let nsec = timestamp % 1000000000;

		Self {
			tv_sec: sec,
			tv_nsec: nsec as _,
		}
	}

	fn to_nano(&self) -> u64 {
		self.tv_sec * 1000000000 + self.tv_nsec as u64
	}
}

impl Add<Timespec> for Timespec {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec + rhs.tv_sec,
			tv_nsec: self.tv_nsec + rhs.tv_nsec,
		}
	}
}

impl PartialEq for Timespec {
	fn eq(&self, other: &Self) -> bool {
		self.tv_sec == other.tv_sec && self.tv_nsec == other.tv_nsec
	}
}

impl PartialOrd for Timespec {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.tv_sec.cmp(&other.tv_sec).then_with(|| self.tv_nsec.cmp(&other.tv_nsec)))
	}
}
