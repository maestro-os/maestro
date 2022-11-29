//! This module implements file descriptors-related features.
//! A file descriptor is an ID held by a process pointing to an entry in the
//! open file description table.

use crate::errno::Errno;
use crate::file::FileLocation;
use crate::file::open_file::OpenFile;
use crate::util::FailableClone;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = 4294967295;

/// File descriptor flag: If set, the file descriptor is closed on successful
/// call to `execve`.
pub const FD_CLOEXEC: i32 = 1;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
/// If the maximum amount of file descriptors is reached, the function does
/// nothing and returns an error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let guard = TOTAL_FD.lock();

	if *guard.get() >= TOTAL_MAX_FD {
		return Err(errno!(ENFILE));
	}
	*guard.get_mut() += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	let guard = TOTAL_FD.lock();
	*guard.get_mut() -= 1;
}

/// Constraints to be respected when creating a new file descriptor.
#[derive(Debug)]
pub enum NewFDConstraint {
	/// No constraint
	None,
	/// The new file descriptor must have given fixed value
	Fixed(u32),
	/// The new file descriptor must have at least the given value
	Min(u32),
}

/// Structure representing a file descriptor.
pub struct FileDescriptor {
	/// The FD's id.
	id: u32,
	/// The FD's flags.
	flags: i32,

	/// The location of the open file.
	location: FileLocation,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	///
	/// If no open file description is associated with the given location, the function creates
	/// one.
	///
	/// Arguments:
	/// - `id` is the ID of the file descriptor.
	/// - `flags` is the set of flags associated with the file descriptor.
	/// - `location` is the location of the open file the file descriptor points to.
	pub fn new(id: u32, flags: i32, location: FileLocation) -> Result<Self, Errno> {
		OpenFile::open(location.clone(), flags)?;

		Ok(Self {
			id,
			flags,

			location,
		})
	}

	/// Returns the file descriptor's ID.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the file descriptor's flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Sets the file descriptor's flags.
	pub fn set_flags(&mut self, flags: i32) {
		self.flags = flags;
	}

	/// Returns the location of the open file the file descriptor points to.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Returns the open file associated with the descriptor.
	pub fn get_open_file(&self) -> SharedPtr<OpenFile> {
		// Unwrap won't fail since open files are closed only when the corresponding file
		// descriptors are all closed
		OpenFile::get(&self.location).unwrap()
	}
}

impl FailableClone for FileDescriptor {
	fn failable_clone(&self) -> Result<Self, Errno> {
		Self::new(self.id, self.flags, self.location.clone())
	}
}

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		OpenFile::close(&self.location);
	}
}
