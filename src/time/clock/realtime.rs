//! Module implementing the realtime clock.

use super::Clock;
use crate::time::Timestamp;
use crate::time::TimestampScale;

/// A settable system-wide clock that measures real time.
pub struct ClockRealtime {
	/// Tells whether the clock is monotonic.
	monotonic: bool,

	/// The current timestamp in nanoseconds.
	//time: AtomicU64, // TODO create wrapper that replace atomic by a mutex on 32 bits platforms
	time: Timestamp,
}

impl ClockRealtime {
	/// Creates a new instance.
	pub fn new(monotonic: bool) -> Self {
		Self {
			monotonic,

			time: 0,
		}
	}
}

impl Clock for ClockRealtime {
	fn is_monotonic(&self) -> bool {
		self.monotonic
	}

	fn get(&self, scale: TimestampScale) -> Timestamp {
		// TODO implement monotonic clock
		TimestampScale::convert(self.time, TimestampScale::Nanosecond, scale)
	}

	fn update(&self, _delta: Timestamp) {
		// TODO implement monotonic clock
		//self.time += delta;
	}
}
