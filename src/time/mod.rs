//! This module handles time-releated features.
//! The kernel stores a list of clock sources. A clock source is an object that allow to get the
//! current timestamp.

use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util::lock::*;

pub mod cmos;

/// Type representing a timestamp.
pub type Timestamp = u32;

/// Trait representing a source able to provide the current timestamp.
pub trait ClockSource {
	/// The name of the source.
	fn get_name(&self) -> &str;
	/// Returns the current timestamp in seconds.
	fn get_time(&mut self) -> Timestamp;
}

/// Vector containing all the clock sources.
static CLOCK_SOURCES: Mutex<Vec<Box<dyn ClockSource>>> = Mutex::new(Vec::new());

/// Returns a reference to the list of clock sources.
pub fn get_clock_sources() -> &'static Mutex<Vec<Box<dyn ClockSource>>> {
	&CLOCK_SOURCES
}

/// Adds the new clock source to the clock sources list.
pub fn add_clock_source<T: 'static + ClockSource>(source: T) -> Result<(), Errno> {
	let mut guard = CLOCK_SOURCES.lock();
	let sources = guard.get_mut();
	sources.push(Box::new(source)?)?;
	Ok(())
}

/// Returns the current timestamp from the preferred clock source.
pub fn get() -> Timestamp {
	let mut guard = CLOCK_SOURCES.lock();
	let sources = guard.get_mut();
	if sources.is_empty() {
		crate::kernel_panic!("No clock source available!");
	}

	let cmos = &mut sources[0]; // TODO Select the preferred source
	cmos.get_time()
}
