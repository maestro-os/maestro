//! This module implements system clocks.

use super::AtomicTimestamp;
use crate::errno::EResult;
use crate::time::unit::ClockIdT;
use crate::time::unit::TimeUnit;
use crate::time::Timestamp;
use crate::time::TimestampScale;
use core::cmp::max;

/// System clock ID
pub const CLOCK_REALTIME: ClockIdT = 0;
/// System clock ID
pub const CLOCK_MONOTONIC: ClockIdT = 1;
/// System clock ID
pub const CLOCK_PROCESS_CPUTIME_ID: ClockIdT = 2;
/// System clock ID
pub const CLOCK_THREAD_CPUTIME_ID: ClockIdT = 3;
/// System clock ID
pub const CLOCK_MONOTONIC_RAW: ClockIdT = 4;
/// System clock ID
pub const CLOCK_REALTIME_COARSE: ClockIdT = 5;
/// System clock ID
pub const CLOCK_MONOTONIC_COARSE: ClockIdT = 6;
/// System clock ID
pub const CLOCK_BOOTTIME: ClockIdT = 7;
/// System clock ID
pub const CLOCK_REALTIME_ALARM: ClockIdT = 8;
/// System clock ID
pub const CLOCK_BOOTTIME_ALARM: ClockIdT = 9;
/// System clock ID
pub const CLOCK_SGI_CYCLE: ClockIdT = 10;
/// System clock ID
pub const CLOCK_TAI: ClockIdT = 11;

// TODO allow accessing clocks through an address shared with userspace (vDSO)

/// The current timestamp of the real time clock, in nanoseconds.
static REALTIME: AtomicTimestamp = AtomicTimestamp::new(0);
/// On time adjustement, this value is updated with the previous value of the real time clock so
/// that it can be used if the clock went backwards in time.
static MONOTONIC: AtomicTimestamp = AtomicTimestamp::new(0);
/// The time elapsed since boot time, in nanoseconds.
static BOOTTIME: AtomicTimestamp = AtomicTimestamp::new(0);

/// Updates clocks with the given delta value in nanoseconds.
pub fn update(delta: Timestamp) {
	REALTIME.fetch_add(delta as _);
	MONOTONIC.fetch_add(delta as _);
	BOOTTIME.fetch_add(delta as _);
}

/// Returns the current timestamp according to the clock with the given ID.
///
/// Arguments:
/// - `clk` is the ID of the clock to use.
/// - `scale` is the scale of the timestamp to return.
///
/// If the clock is invalid, the function returns an error.
pub fn current_time(clk: ClockIdT, scale: TimestampScale) -> EResult<Timestamp> {
	// TODO implement all clocks
	let raw_ts = match clk {
		CLOCK_REALTIME | CLOCK_REALTIME_ALARM => REALTIME.load(),
		CLOCK_MONOTONIC => {
			let realtime = REALTIME.load();
			let monotonic = MONOTONIC.load();

			max(realtime, monotonic)
		}
		CLOCK_BOOTTIME | CLOCK_BOOTTIME_ALARM => BOOTTIME.load(),

		_ => return Err(errno!(EINVAL)),
	};

	Ok(TimestampScale::convert(
		raw_ts as _,
		TimestampScale::Nanosecond,
		scale,
	))
}

/// Returns the current timestamp according to the clock with the given ID.
///
/// Arguments:
/// - `clk` is the ID of the clock to use.
/// - `scale` is the scale of the timestamp to return.
///
/// If the clock is invalid, the function returns an error.
pub fn current_time_struct<T: TimeUnit>(clk: ClockIdT) -> EResult<T> {
	let ts = current_time(clk, TimestampScale::Nanosecond)?;
	Ok(T::from_nano(ts))
}
