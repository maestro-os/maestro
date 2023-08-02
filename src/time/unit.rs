//! This module implements types representing timestamps.

use core::cmp::Ordering;
use core::ffi::c_int;
use core::ffi::c_long;
use core::ffi::c_void;
use core::ops::Add;
use core::ops::Sub;

/// Type representing a timestamp in seconds. Equivalent to POSIX's `time_t`.
pub type Timestamp = u64;
/// Type representing a timestamp in microseconds. Equivalent to POSIX's
/// `suseconds_t`.
pub type UTimestamp = u64;
/// Equivalent of POSIX `clockid_t`.
pub type ClockIdT = c_int;
/// Equivalent of POSIX `timer_t`.
pub type TimerT = *mut c_void;

/// Enumeration of available timestamp scales.
#[derive(Debug)]
pub enum TimestampScale {
	/// The unit is one second.
	Second,
	/// The unit is one millisecond.
	Millisecond,
	/// The unit is one microsecond.
	Microsecond,
	/// The unit is one nanosecond.
	Nanosecond,
}

impl TimestampScale {
	/// Returns the order of the scale as a power of `10`.
	pub fn as_power(&self) -> u32 {
		match self {
			Self::Second => 0,
			Self::Millisecond => 3,
			Self::Microsecond => 6,
			Self::Nanosecond => 9,
		}
	}

	/// Converts the given value `val` from scale `from` to scale `to`.
	pub fn convert(val: Timestamp, from: Self, to: Self) -> Timestamp {
		let to_power = to.as_power();
		let from_power = from.as_power();

		if to_power > from_power {
			val * 10_u64.pow(to_power - from_power)
		} else {
			val / 10_u64.pow(from_power - to_power)
		}
	}
}

/// Trait to be implement on a structure describing a moment in time.
pub trait TimeUnit:
	Sized + Clone + Default + Add<Self, Output = Self> + Sub<Self, Output = Self> + PartialOrd
{
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
#[derive(Clone, Copy, Debug, Default)]
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
		self.tv_sec
			.wrapping_mul(1000000000)
			.wrapping_add(self.tv_usec.wrapping_mul(1000))
	}

	fn is_zero(&self) -> bool {
		self.tv_sec == 0 && self.tv_usec == 0
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

impl Sub<Timeval> for Timeval {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec - rhs.tv_sec,
			tv_usec: self.tv_usec - rhs.tv_usec,
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
		Some(
			self.tv_sec
				.cmp(&other.tv_sec)
				.then_with(|| self.tv_usec.cmp(&other.tv_usec)),
		)
	}
}

/// Same as `Timeval`, but with nanosecond precision.
#[derive(Clone, Copy, Debug, Default, Eq, Ord)]
#[repr(C)]
pub struct Timespec {
	/// Seconds
	pub tv_sec: Timestamp,
	/// Nanoseconds
	pub tv_nsec: c_long,
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
		self.tv_sec
			.wrapping_mul(1000000000)
			.wrapping_add(self.tv_nsec as u64)
	}

	fn is_zero(&self) -> bool {
		self.tv_sec == 0 && self.tv_nsec == 0
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

impl Sub<Timespec> for Timespec {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec - rhs.tv_sec,
			tv_nsec: self.tv_nsec - rhs.tv_nsec,
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
		Some(
			self.tv_sec
				.cmp(&other.tv_sec)
				.then_with(|| self.tv_nsec.cmp(&other.tv_nsec)),
		)
	}
}

/// Same as `Timespec`, but with 32 bits values.
#[derive(Clone, Copy, Debug, Default, Eq, Ord)]
#[repr(C)]
pub struct Timespec32 {
	/// Seconds
	pub tv_sec: u32,
	/// Nanoseconds
	pub tv_nsec: u32,
}

impl TimeUnit for Timespec32 {
	fn from_nano(timestamp: u64) -> Self {
		let sec = timestamp / 1000000000;
		let nsec = timestamp % 1000000000;

		Self {
			tv_sec: sec as _,
			tv_nsec: nsec as _,
		}
	}

	fn to_nano(&self) -> u64 {
		(self.tv_sec as u64)
			.wrapping_mul(1000000000)
			.wrapping_add(self.tv_nsec as u64)
	}

	fn is_zero(&self) -> bool {
		self.tv_sec == 0 && self.tv_nsec == 0
	}
}

impl Add<Timespec32> for Timespec32 {
	type Output = Self;

	fn add(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec + rhs.tv_sec,
			tv_nsec: self.tv_nsec + rhs.tv_nsec,
		}
	}
}

impl Sub<Timespec32> for Timespec32 {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self {
		Self {
			tv_sec: self.tv_sec - rhs.tv_sec,
			tv_nsec: self.tv_nsec - rhs.tv_nsec,
		}
	}
}

impl PartialEq for Timespec32 {
	fn eq(&self, other: &Self) -> bool {
		self.tv_sec == other.tv_sec && self.tv_nsec == other.tv_nsec
	}
}

impl PartialOrd for Timespec32 {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(
			self.tv_sec
				.cmp(&other.tv_sec)
				.then_with(|| self.tv_nsec.cmp(&other.tv_nsec)),
		)
	}
}

/// Structure specifying a timer's state.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ITimerspec32 {
	/// The interval between each firing of the timer.
	pub it_interval: Timespec32,
	/// Start value of the timer.
	pub it_value: Timespec32,
}
