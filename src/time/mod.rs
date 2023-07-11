//! This module handles time-releated features.
//!
//! The kernel stores a list of clock sources. A clock source is an object that
//! allow to get the current timestamp.

pub mod clock;
pub mod timer;
pub mod unit;

use crate::errno::EResult;
use crate::errno::Errno;
use crate::time::clock::CLOCK_MONOTONIC;
use crate::time::clock::CLOCK_REALTIME;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::lock::*;
use unit::Timestamp;
use unit::TimestampScale;

/// Trait representing a source able to provide the current timestamp.
pub trait ClockSource {
	/// The name of the source.
	fn get_name(&self) -> &'static str;
	/// Returns the current timestamp in seconds.
	/// `scale` specifies the scale of the returned timestamp.
	fn get_time(&mut self, scale: TimestampScale) -> Timestamp;
}

/// Structure wrapping a clock source.
struct ClockSourceWrapper {
	/// The clock source.
	src: Box<dyn ClockSource>,

	/// The last timestamp returned by the clock.
	///
	/// This timestamp is used in case the caller requires monotonic time and the clock came back
	/// in the past.
	last: [Timestamp; 4],
}

// TODO use array instead (IDs are contiguous)
/// Map containing all the clock sources.
static CLOCK_SOURCES: Mutex<HashMap<String, ClockSourceWrapper>> = Mutex::new(HashMap::new());

/// Adds the new clock source to the clock sources list.
pub fn add_clock_source<T: 'static + ClockSource>(source: T) -> Result<(), Errno> {
	let mut sources = CLOCK_SOURCES.lock();

	let name = String::try_from(source.get_name())?;
	sources.insert(
		name,
		ClockSourceWrapper {
			src: Box::new(source)?,

			last: [0; 4],
		},
	)?;

	Ok(())
}

/// Removes the clock source with the given name.
///
/// If the clock source doesn't exist, the function does nothing.
pub fn remove_clock_source(name: &str) {
	let mut sources = CLOCK_SOURCES.lock();
	sources.remove(name.as_bytes());
}

/// Initializes clocks.
pub fn init() -> EResult<()> {
	// Initializes PIT
	timer::pit::init();

	// Initializes clocks
	let mut clocks = clock::CLOCKS.lock();
	clocks.insert(
		CLOCK_REALTIME,
		Box::new(clock::realtime::ClockRealtime::new(false))?,
	)?;
	clocks.insert(
		CLOCK_MONOTONIC,
		Box::new(clock::realtime::ClockRealtime::new(true))?,
	)?;
	// TODO register all

	Ok(())
}
