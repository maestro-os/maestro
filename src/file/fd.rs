//! This module implements file descriptors-related features.
//! A file descriptor is an ID held by a process pointing to an entry in the open file description
//! table.

use crate::errno::Errno;
use crate::util::lock::Mutex;

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = 4294967295;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
/// If the maximum amount of file descriptors is reached, the function does nothing and returns an
/// error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let mut guard = TOTAL_FD.lock();

	if *guard.get() >= TOTAL_MAX_FD {
		return Err(errno!(ENFILE));
	}
	*guard.get_mut() += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	let mut guard = TOTAL_FD.lock();
	*guard.get_mut() -= 1;
}

/// Constraints to be respected when creating a new file descriptor.
pub enum NewFDConstraint {
	/// No constraint
	None,
	/// The new file descriptor must have given fixed value
	Fixed(u32),
	/// The new file descriptor must have at least the given value
	Min(u32),
}
