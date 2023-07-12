//! Module implementing the realtime clock.

use super::Clock;
use crate::time::Timestamp;
use crate::time::TimestampScale;
use core::sync::atomic;
use core::sync::atomic::AtomicU32;

/// A settable system-wide clock that measures real time.
pub struct ClockRealtime {
	/// Tells whether the clock is monotonic.
	monotonic: bool,

	// TODO use AtomicU64 instead, but since it is not available on 32 bits platform, create a
	// wrapper
	/// The current timestamp in nanoseconds.
	time: AtomicU32,
}

impl ClockRealtime {
	/// Creates a new instance.
	pub fn new(monotonic: bool) -> Self {
		Self {
			monotonic,

			time: AtomicU32::new(0),
		}
	}
}

impl Clock for ClockRealtime {
	fn get(&self, scale: TimestampScale) -> Timestamp {
		// TODO implement monotonic clock

		let val = self.time.load(atomic::Ordering::Relaxed);
		TimestampScale::convert(val as _, TimestampScale::Nanosecond, scale)
	}

	fn update(&self, delta: Timestamp) {
		// TODO implement monotonic clock

		self.time.fetch_add(delta as _, atomic::Ordering::Relaxed);
	}
}
