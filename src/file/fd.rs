//! This module implements file descriptors-related features.
//!
//! A file descriptor is an ID held by a process pointing to an entry in the
//! open file description table.

use crate::errno::CollectResult;
use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::open_file::OpenFile;
use crate::file::FileLocation;
use crate::limits;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryClone;
use core::cmp::max;

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = 4294967295;

/// File descriptor flag: If set, the file descriptor is closed on successful
/// call to `execve`.
pub const FD_CLOEXEC: i32 = 1;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
///
/// If the maximum amount of file descriptors is reached, the function does
/// nothing and returns an error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let mut total_fd = TOTAL_FD.lock();

	if *total_fd >= TOTAL_MAX_FD {
		return Err(errno!(ENFILE));
	}
	*total_fd += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	*TOTAL_FD.lock() -= 1;
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
#[derive(Debug)]
pub struct FileDescriptor {
	/// The FD's id.
	id: u32,
	/// The FD's flags.
	flags: i32,

	/// Tells whether the file descriptor is open for reading.
	read: bool,
	/// Tells whether the file descriptor is open for writing.
	write: bool,

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
	/// - `read` tells whether the file descriptor is open for reading.
	/// - `write` tells whether the file descriptor is open for writing.
	/// - `location` is the location of the open file the file descriptor points to.
	pub fn new(
		id: u32,
		flags: i32,
		read: bool,
		write: bool,
		location: FileLocation,
	) -> Result<Self, Errno> {
		OpenFile::open(location.clone(), read, write)?;

		Ok(Self {
			id,
			flags,

			read,
			write,

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

	/// Tells whether the file descriptor is open for reading.
	pub fn can_read(&self) -> bool {
		self.read
	}

	/// Tells whether the file descriptor is open for writing.
	pub fn can_write(&self) -> bool {
		self.write
	}

	/// Returns the location of the open file the file descriptor points to.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Returns the open file associated with the descriptor.
	///
	/// If the open file doesn't exist, the function returns an error.
	pub fn get_open_file(&self) -> Result<Arc<Mutex<OpenFile>>, Errno> {
		OpenFile::get(&self.location).ok_or_else(|| errno!(ENOENT))
	}
}

impl TryClone for FileDescriptor {
	type Error = Errno;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		Self::new(
			self.id,
			self.flags,
			self.read,
			self.write,
			self.location.clone(),
		)
	}
}

impl IO for FileDescriptor {
	fn get_size(&self) -> u64 {
		if let Ok(open_file_mutex) = self.get_open_file() {
			let open_file = open_file_mutex.lock();
			open_file.get_size()
		} else {
			0
		}
	}

	fn read(&mut self, off: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		if !self.can_read() {
			return Err(errno!(EBADF));
		}

		let open_file_mutex = self.get_open_file()?;
		let mut open_file = open_file_mutex.lock();

		open_file.read(off, buf)
	}

	fn write(&mut self, off: u64, buf: &[u8]) -> Result<u64, Errno> {
		if !self.can_write() {
			return Err(errno!(EBADF));
		}

		let open_file_mutex = self.get_open_file()?;
		let mut open_file = open_file_mutex.lock();

		open_file.write(off, buf)
	}

	fn poll(&mut self, mask: u32) -> Result<u32, Errno> {
		let open_file_mutex = self.get_open_file()?;
		let mut open_file = open_file_mutex.lock();

		open_file.poll(mask)
	}
}

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		let _ = OpenFile::close(&self.location, self.read, self.write);
		// TODO print error? panic?
	}
}

/// A table of file descriptors.
#[derive(Debug)]
pub struct FileDescriptorTable {
	/// The list of file descriptors.
	fds: Vec<FileDescriptor>,
}

impl FileDescriptorTable {
	/// Returns the available file descriptor with the lowest ID.
	///
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
	/// - `read` tells whether the file descriptor is open for reading.
	/// - `write` tells whether the file descriptor is open for writing.
	pub fn create_fd(
		&mut self,
		location: FileLocation,
		flags: i32,
		read: bool,
		write: bool,
	) -> Result<&FileDescriptor, Errno> {
		let id = self.get_available_fd(None)?;
		let i = self
			.fds
			.binary_search_by(|fd| fd.get_id().cmp(&id))
			.unwrap_err();

		let fd = FileDescriptor::new(id, flags, read, write, location)?;
		self.fds.insert(i, fd)?;

		Ok(&self.fds[i])
	}

	/// Returns an immutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor doesn't exist, the function returns `None`.
	pub fn get_fd(&self, id: u32) -> Option<&FileDescriptor> {
		let result = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id));
		result.ok().map(|index| &self.fds[index])
	}

	/// Returns a mutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor doesn't exist, the function returns `None`.
	pub fn get_fd_mut(&mut self, id: u32) -> Option<&mut FileDescriptor> {
		let result = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id));
		result.ok().map(|index| &mut self.fds[index])
	}

	/// Duplicates the file descriptor with id `id`.
	///
	/// Arguments:
	/// - `constraint` is the constraint the new file descriptor ID willl follows.
	/// - `cloexec` tells whether the new file descriptor has the `FD_CLOEXEC` flag enabled.
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

		// The old FD
		let old_fd = self.get_fd(id).ok_or_else(|| errno!(EBADF))?;

		// Creating the new FD
		let new_fd = FileDescriptor::new(
			new_id,
			flags,
			old_fd.can_read(),
			old_fd.can_write(),
			old_fd.get_location().clone(),
		)?;

		// Inserting the FD
		let index = self.fds.binary_search_by(|fd| fd.get_id().cmp(&new_id));
		let index = {
			if let Ok(i) = index {
				self.fds[i] = new_fd;
				i
			} else {
				let i = index.unwrap_err();
				self.fds.insert(i, new_fd)?;
				i
			}
		};

		Ok(&self.fds[index])
	}

	/// Duplicates the whole file descriptors table.
	///
	/// `cloexec` specifies whether the cloexec file must be taken into account. This is the case
	/// when executing a program.
	pub fn duplicate(&self, cloexec: bool) -> EResult<Self> {
		let fds = self
			.fds
			.iter()
			.filter(|fd| {
				// cloexec implies fd's cloexec flag must be clear
				!cloexec || fd.get_flags() & FD_CLOEXEC == 0
			})
			.map(FileDescriptor::try_clone)
			.collect::<EResult<CollectResult<Vec<_>>>>()?
			.0?;
		Ok(Self {
			fds,
		})
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
