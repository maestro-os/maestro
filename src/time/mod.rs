//! This module handles time-releated features.
//! The kernel stores a list of clock sources. A clock source is an object that allow to get the
//! current timestamp.

pub mod timer;
pub mod unit;

use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::lock::*;
use unit::TimeUnit;
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
	last: Timestamp,
}

/// Map containing all the clock sources.
static CLOCK_SOURCES: Mutex<HashMap<String, ClockSourceWrapper>> = Mutex::new(HashMap::new());

/// Adds the new clock source to the clock sources list.
pub fn add_clock_source<T: 'static + ClockSource>(source: T) -> Result<(), Errno> {
	let guard = CLOCK_SOURCES.lock();
	let sources = guard.get_mut();

	let name = String::from(source.get_name().as_bytes())?;
	sources.insert(name, ClockSourceWrapper {
		src: Box::new(source)?,

		last: 0,
	})?;

	Ok(())
}

/// Removes the clock source with the given name.
/// If the clock source doesn't exist, the function does nothing.
pub fn remove_clock_source(name: &str) {
	let guard = CLOCK_SOURCES.lock();
	let sources = guard.get_mut();

	sources.remove(name.as_bytes());
}

/// Returns the current timestamp from the preferred clock source.
/// `scale` specifies the scale of the returned timestamp.
/// `monotonic` tells whether the returned time should be monotonic.
/// If no clock source is available, the function returns None.
pub fn get(scale: TimestampScale, monotonic: bool) -> Option<Timestamp> {
	let guard = CLOCK_SOURCES.lock();
	let sources = guard.get_mut();

	if sources.is_empty() {
		return None;
	}

	// Getting clock source
	let clock_src = sources.get_mut("cmos".as_bytes())?; // TODO Select the preferred source
	// Getting time
	let time = clock_src.src.get_time(scale);

	// Making the clock monotonic if needed
	let ts = if monotonic && clock_src.last > time {
		clock_src.last
	} else {
		time
	};
	if ts > clock_src.last {
		clock_src.last = ts;
	}

	Some(ts)
}

/// Returns the current timestamp from the given clock `clk`.
/// `scale` specifies the scale of the returned timestamp.
/// `monotonic` tells whether the returned time should be monotonic.
/// If the clock doesn't exist, the function returns None.
pub fn get_struct<T: TimeUnit>(_clk: &[u8], monotonic: bool) -> Option<T> {
	// TODO use the given clock
	let ts = get(TimestampScale::Nanosecond, monotonic)?;
	Some(T::from_nano(ts))
}

/// Makes the CPU wait for at least `n` milliseconds.
pub fn mdelay(n: u32) {
	// TODO
	udelay(n * 1000);
}

/// Makes the CPU wait for at least `n` microseconds.
pub fn udelay(n: u32) {
	// TODO
	for _ in 0..(n * 100) {
		unsafe {
			core::arch::asm!("nop");
		}
	}
}

/// Makes the CPU wait for at least `n` nanoseconds.
pub fn ndelay(n: u32) {
	// TODO
	udelay(n);
}
