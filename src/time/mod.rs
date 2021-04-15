/// This module handles time-releated features.
/// TODO doc

pub mod cmos;

/// Type representing a timestamp.
pub type Timestamp = u32;

/// Trait representing a source able to provide the current timestamp.
pub trait ClockSource {
	/// The name of the source.
	fn get_name(&self) -> &str;
	/// Returns the current timestamp in seconds.
	fn get_time(&self) -> Timestamp;
}

// TODO Function to get the clock source list
// TODO Function to get a clock source by name
// TODO Function to add a clock source

/// Returns the current timestamp for the preferred clock source.
pub fn get() -> Timestamp {
	// TODO Use a list of clock sources
	let cmos = cmos::CMOSClock::new(false);
	cmos.get_time()
}
