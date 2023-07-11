//! Module implementing the realtime clock.

use super::Clock;
use crate::time::Timestamp;
use crate::time::TimestampScale;

/// A settable system-wide clock that measures real time.
#[derive(Default)]
pub struct ClockRealtime {}

impl Clock for ClockRealtime {
	fn is_monotonic(&self) -> bool {
		false
	}

	fn get(&self, _scale: TimestampScale) -> Timestamp {
		// TODO
		todo!()
	}
}
