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

//! Userspace types representing timestamps.

use core::{
	cmp::Ordering,
	ffi::{c_int, c_long},
	fmt::Debug,
	ops::{Add, Sub},
};

/// Type representing a timestamp in seconds. Equivalent to POSIX's `time_t`.
pub type Timestamp = u64;
/// Type representing a timestamp in microseconds. Equivalent to POSIX's
/// `suseconds_t`.
pub type UTimestamp = u64;
/// Equivalent of POSIX `clockid_t`.
pub type ClockIdT = c_int;
// FIXME: size must match the size of a pointer in userspace
/// Equivalent of POSIX `timer_t`.
pub type TimerT = usize;

/// A structure describing a timestamp in userspace.
pub trait TimeUnit: Sized + Clone + Copy + Debug {
	/// Creates the structure from the given timestamp in nanoseconds.
	fn from_nano(timestamp: u64) -> Self;
	/// Returns the equivalent timestamp in nanoseconds.
	fn to_nano(&self) -> u64;
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
		Self {
			tv_sec: timestamp / 1_000_000_000,
			tv_usec: (timestamp % 1_000_000_000) / 1000,
		}
	}

	fn to_nano(&self) -> u64 {
		self.tv_sec
			.wrapping_mul(1_000_000_000)
			.wrapping_add(self.tv_usec.wrapping_mul(1000))
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

/// Same as [`Timeval`], but with nanosecond precision.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Timespec {
	/// Seconds
	pub tv_sec: Timestamp,
	/// Nanoseconds
	pub tv_nsec: c_long,
}

impl TimeUnit for Timespec {
	fn from_nano(timestamp: u64) -> Self {
		Self {
			tv_sec: timestamp / 1_000_000_000,
			tv_nsec: (timestamp % 1_000_000_000) as _,
		}
	}

	fn to_nano(&self) -> u64 {
		self.tv_sec
			.wrapping_mul(1_000_000_000)
			.wrapping_add(self.tv_nsec as u64)
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

impl Ord for Timespec {
	fn cmp(&self, other: &Self) -> Ordering {
		self.tv_sec
			.cmp(&other.tv_sec)
			.then_with(|| self.tv_nsec.cmp(&other.tv_nsec))
	}
}

impl PartialOrd for Timespec {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

/// Same as [`Timespec`], but with 32 bits values.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Timespec32 {
	/// Seconds
	pub tv_sec: u32,
	/// Nanoseconds
	pub tv_nsec: u32,
}

impl TimeUnit for Timespec32 {
	fn from_nano(timestamp: u64) -> Self {
		Self {
			tv_sec: (timestamp / 1_000_000_000) as _,
			tv_nsec: (timestamp % 1_000_000_000) as _,
		}
	}

	fn to_nano(&self) -> u64 {
		(self.tv_sec as u64)
			.wrapping_mul(1_000_000_000)
			.wrapping_add(self.tv_nsec as u64)
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

impl Ord for Timespec32 {
	fn cmp(&self, other: &Self) -> Ordering {
		self.tv_sec
			.cmp(&other.tv_sec)
			.then_with(|| self.tv_nsec.cmp(&other.tv_nsec))
	}
}

impl PartialOrd for Timespec32 {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

/// A timer's state.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ITimerspec32 {
	/// The interval between each firing of the timer.
	pub it_interval: Timespec32,
	/// Start value of the timer.
	pub it_value: Timespec32,
}
