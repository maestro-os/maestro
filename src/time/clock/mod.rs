//! This module implements system clocks.

pub(super) mod realtime;

use crate::time::unit::ClockIdT;
use crate::time::unit::TimeUnit;
use crate::time::Timestamp;
use crate::time::TimestampScale;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;

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

// TODO allow accessing clocks:
// - without locking a Mutex (atomic exchange)
// - through an address shared with userspace (vDSO)

/// Trait representing a system clock.
pub trait Clock {
	/// Tells whether the clock is monotonic.
	fn is_monotonic(&self) -> bool;

	/// Returns the clock's current timestamp.
	fn get(&self, scale: TimestampScale) -> Timestamp;
}

/// The list of system clocks.
pub static CLOCKS: Mutex<HashMap<ClockIdT, Box<dyn Clock>>> = Mutex::new(HashMap::new());

/// Returns the current timestamp according to the clock with the given ID.
///
/// Arguments:
/// - `clk` is the ID of the clock to use.
/// - `scale` is the scale of the timestamp to return.
pub fn current_time(_clk: ClockIdT, _scale: TimestampScale) -> Timestamp {
	// TODO
	todo!()
}

/// Returns the current timestamp according to the clock with the given ID.
///
/// Arguments:
/// - `clk` is the ID of the clock to use.
/// - `scale` is the scale of the timestamp to return.
pub fn current_time_struct<T: TimeUnit>(clk: ClockIdT) -> T {
	let ts = current_time(clk, TimestampScale::Nanosecond);
	T::from_nano(ts)
}
