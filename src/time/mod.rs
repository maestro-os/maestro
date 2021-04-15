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

// TODO Function to get the clock source list or to get a clock source by name
