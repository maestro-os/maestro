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

//! System clocks.

use crate::{
	sync::atomic::AtomicU64,
	time::{unit::ClockIdT, Timestamp},
};
use core::{
	cmp::max,
	sync::atomic::Ordering::{Acquire, Release},
};

/// Available clocks
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub enum Clock {
	Realtime = 0,
	Monotonic = 1,
	ProcessCputimeId = 2,
	ThreadCputimeId = 3,
	MonotonicRaw = 4,
	RealtimeCoarse = 5,
	MonotonicCoarse = 6,
	Boottime = 7,
	RealtimeAlarm = 8,
	BoottimeAlarm = 9,
	SgiCycle = 10,
	Tai = 11,
}

impl Clock {
	/// Returns the clock with the given ID.
	///
	/// If the ID is invalid, the function returns `None`.
	pub fn from_id(id: ClockIdT) -> Option<Self> {
		match id {
			0 => Some(Self::Realtime),
			1 => Some(Self::Monotonic),
			2 => Some(Self::ProcessCputimeId),
			3 => Some(Self::ThreadCputimeId),
			4 => Some(Self::MonotonicRaw),
			5 => Some(Self::RealtimeCoarse),
			6 => Some(Self::MonotonicCoarse),
			7 => Some(Self::Boottime),
			8 => Some(Self::RealtimeAlarm),
			9 => Some(Self::BoottimeAlarm),
			10 => Some(Self::SgiCycle),
			11 => Some(Self::Tai),
			_ => None,
		}
	}
}

// TODO allow accessing clocks through an address shared with userspace (vDSO)

/// The current timestamp of the real time clock, in nanoseconds.
static REALTIME: AtomicU64 = AtomicU64::new(0);
/// On time adjustment, this value is updated with the previous value of the real time clock so
/// that it can be used if the clock went backwards in time.
static MONOTONIC: AtomicU64 = AtomicU64::new(0);
/// The time elapsed since boot time, in nanoseconds.
static BOOTTIME: AtomicU64 = AtomicU64::new(0);

/// Updates clocks with the given delta value in nanoseconds.
pub fn update(delta: Timestamp) {
	REALTIME.fetch_add(delta as _, Release);
	MONOTONIC.fetch_add(delta as _, Release);
	BOOTTIME.fetch_add(delta as _, Release);
}

/// Returns the current timestamp in nanoseconds.
///
/// `clk` is the clock to use.
///
/// The returned timestamp is in nanoseconds.
///
/// If the clock is invalid, the function returns an error.
pub fn current_time_ns(clk: Clock) -> Timestamp {
	match clk {
		Clock::Realtime | Clock::RealtimeAlarm => REALTIME.load(Acquire),
		Clock::Monotonic => {
			let realtime = REALTIME.load(Acquire);
			let monotonic = MONOTONIC.load(Acquire);
			max(realtime, monotonic)
		}
		Clock::Boottime | Clock::BoottimeAlarm => BOOTTIME.load(Acquire),
		// TODO implement all clocks
		_ => 0,
	}
}

/// Returns the current timestamp in milliseconds.
///
/// `clk` is the clock to use.
#[inline]
pub fn current_time_ms(clk: Clock) -> Timestamp {
	current_time_ns(clk) / 1_000_000
}

/// Returns the current timestamp in seconds.
///
/// `clk` is the clock to use.
#[inline]
pub fn current_time_sec(clk: Clock) -> Timestamp {
	current_time_ns(clk) / 1_000_000_000
}
