//! This module implements file descriptors-related features.
//! A file descriptor is an ID held by a process pointing to an entry in the
//! open file description table.

// TODO Maintain the system-wide open file descriptors count

use core::cmp::max;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::file::open_file::O_CLOEXEC;
use crate::file::open_file::OpenFile;
use crate::limits;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;
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

/// A table of file descriptors.
pub struct FileDescriptorTable {
	/// The list of file descriptors.
	fds: Vec<FileDescriptor>,
}

impl FileDescriptorTable {
	/// Returns the available file descriptor with the lowest ID.
	/// If no ID is available, the function returns an error.
	///
	/// `min` is the minimum value for the file descriptor to be returned.
	fn get_available_fd(&self, min: Option<u32>) -> Result<u32, Errno> {
		if self.fds.len() >= limits::OPEN_MAX {
			return Err(errno!(EMFILE));
		}
		if self.fds.is_empty() {
			return Ok(0);
		}

		// TODO Use binary search?
		for (i, fd) in self.fds.iter().enumerate() {
			if let Some(min) = min {
				if fd.get_id() < min {
					continue;
				}
			}

			if (i as u32) < fd.get_id() {
				return Ok(i as u32);
			}
		}

		let id = match min {
			Some(min) => max(min, self.fds.len() as u32),
			None => self.fds.len() as u32,
		};
		Ok(id)
	}

	/// Creates a file descriptor and returns a pointer to it with its ID.
	///
	/// Arguments:
	/// - `location` is the location of the file the newly created file descriptor points to.
	/// - `flags` are the file descriptor's flags.
	pub fn create_fd(
		&mut self,
		location: FileLocation,
		flags: i32
	) -> Result<&FileDescriptor, Errno> {
		let id = self.get_available_fd(None)?;
		let i = self.fds
			.binary_search_by(|fd| fd.get_id().cmp(&id))
			.unwrap_err();

		// Flags for the fd
		let flags = if flags & O_CLOEXEC != 0 {
			FD_CLOEXEC
		} else {
			0
		};

		let fd = FileDescriptor::new(id, flags, location)?;
		self.fds.insert(i, fd)?;

		Ok(&self.fds[i])
	}

	/// Returns an immutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor doesn't exist, the function returns None.
	pub fn get_fd(&self, id: u32) -> Option<&FileDescriptor> {
		let result = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id));
		result.ok().map(|index| &self.fds[index])
	}

	/// Sets the given flags to the given file descriptor.
	///
	/// If the file descriptor doesn't exist, the function returns an error.
	pub fn set_fd_flags(&mut self, id: u32, flags: i32) -> Result<(), Errno> {
		let Ok(index) = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id)) else {
			return Err(errno!(EBADF));
		};

		self.fds[index].set_flags(flags);
		Ok(())
	}

	/// Duplicates the file descriptor with id `id`.
	///
	/// Arguments:
	/// - `constraint` is the constraint the new file descriptor ID willl follows.
	/// - `cloexec` tells whether the new file descriptor has the `O_CLOEXEC` flag enabled.
	///
	/// The function returns a pointer to the file descriptor with its ID.
	pub fn duplicate_fd(
		&mut self,
		id: u32,
		constraint: NewFDConstraint,
		cloexec: bool,
	) -> Result<&FileDescriptor, Errno> {
		// The ID of the new FD
		let new_id = match constraint {
			NewFDConstraint::None => self.get_available_fd(None)?,
			NewFDConstraint::Fixed(id) => id,
			NewFDConstraint::Min(min) => self.get_available_fd(Some(min))?,
		};

		// The flags of the new FD
		let flags = if cloexec { FD_CLOEXEC } else { 0 };

		// The location of the file
		let location = self.get_fd(id)
			.ok_or_else(|| errno!(EBADF))?
			.get_location()
			.clone();

		// Creating the FD
		let fd = FileDescriptor::new(new_id, flags, location)?;

		// Inserting the FD
		let index = self.fds.binary_search_by(|fd| fd.get_id().cmp(&new_id));
		let index = {
			if let Ok(i) = index {
				self.fds[i] = fd;
				i
			} else {
				let i = index.unwrap_err();
				self.fds.insert(i, fd)?;
				i
			}
		};

		Ok(&self.fds[index])
	}

	/// Closes the file descriptor with the ID `id`.
	///
	/// The function returns an Err if the file descriptor doesn't exist.
	pub fn close_fd(&mut self, id: u32) -> Result<(), Errno> {
		let result = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id));

		if let Ok(index) = result {
			self.fds.remove(index);
			Ok(())
		} else {
			Err(errno!(EBADF))
		}
	}
}

impl Default for FileDescriptorTable {
	fn default() -> Self {
		Self {
			fds: Vec::new(),
		}
	}
}

impl FailableClone for FileDescriptorTable {
	fn failable_clone(&self) -> Result<Self, Errno> {
		let mut fds = Vec::new();

		for fd in &self.fds {
			if fd.get_flags() & FD_CLOEXEC == 0 {
				fds.push(fd.failable_clone()?)?;
			}
		}

		Ok(Self {
			fds,
		})
	}
}
