/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements file descriptors-related features.
//!
//! A file descriptor is an ID held by a process pointing to an entry in the
//! open file description table.

use crate::{file::open_file::OpenFile, limits};
use core::{cmp::max, ffi::c_int};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{CollectResult, EResult},
	io::IO,
	lock::Mutex,
	ptr::arc::Arc,
};

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = u32::MAX as usize;

/// File descriptor flag: If set, the file descriptor is closed on successful
/// call to `execve`.
pub const FD_CLOEXEC: i32 = 1;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
///
/// If the maximum amount of file descriptors is reached, the function does
/// nothing and returns an error with the appropriate errno.
fn increment_total() -> EResult<()> {
	let mut total_fd = TOTAL_FD.lock();
	#[allow(clippy::absurd_extreme_comparisons)]
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

/// Constraints on a new file descriptor ID.
#[derive(Debug)]
pub enum NewFDConstraint {
	/// No constraint
	None,
	/// The new file descriptor must have given fixed value
	Fixed(u32),
	/// The new file descriptor must have at least the given value
	Min(u32),
}

/// A file descriptor, pointing to an [`OpenFile`].
#[derive(Clone, Debug)]
pub struct FileDescriptor {
	/// The file descriptor's ID.
	id: u32,
	/// The file descriptor's flags.
	pub flags: i32,
	/// The open file description associated with the file descriptor.
	open_file: Arc<Mutex<OpenFile>>,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	///
	/// If no open file description is associated with the given location, the function creates
	/// one.
	///
	/// Arguments:
	/// - `id` is the ID of the file descriptor
	/// - `flags` is the set of flags associated with the file descriptor
	/// - `location` is the location of the open file the file descriptor points to
	pub fn new(id: u32, flags: i32, open_file: OpenFile) -> EResult<Self> {
		let open_file = Arc::new(Mutex::new(open_file))?;
		Ok(Self {
			id,
			flags,
			open_file,
		})
	}

	/// Returns the file descriptor's ID.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the open file associated with the descriptor.
	pub fn get_open_file(&self) -> &Arc<Mutex<OpenFile>> {
		&self.open_file
	}

	/// Duplicates the file descriptor with the given ID.
	pub fn duplicate(&self, id: u32) -> Self {
		Self {
			id,
			flags: self.flags,
			open_file: self.open_file.clone(),
		}
	}

	/// Closes the file descriptor.
	///
	/// If the file descriptor is the last reference to the underlying open file description, the
	/// function also closes it.
	///
	/// If file removal has been deferred, and this is the last reference to it, and remove fails,
	/// then the function returns an error.
	pub fn close(self) -> EResult<()> {
		// Close file if this is the last reference to it
		let Some(file) = Arc::into_inner(self.open_file) else {
			return Ok(());
		};
		file.into_inner().close()
	}
}

impl IO for FileDescriptor {
	fn get_size(&self) -> u64 {
		self.open_file.lock().get_size()
	}

	fn read(&mut self, off: u64, buf: &mut [u8]) -> EResult<(u64, bool)> {
		self.open_file.lock().read(off, buf)
	}

	fn write(&mut self, off: u64, buf: &[u8]) -> EResult<u64> {
		self.open_file.lock().write(off, buf)
	}

	fn poll(&mut self, mask: u32) -> EResult<u32> {
		self.open_file.lock().poll(mask)
	}
}

// TODO use a BTreeMap or BTreeSet instead?
/// A table of file descriptors.
#[derive(Default)]
pub struct FileDescriptorTable(Vec<FileDescriptor>);

impl FileDescriptorTable {
	/// Returns the available file descriptor with the lowest ID.
	///
	/// If no ID is available, the function returns an error.
	///
	/// `min` is the minimum value for the file descriptor to be returned.
	fn get_available_fd(&self, min: Option<u32>) -> EResult<u32> {
		let min = min.unwrap_or(0);
		if min >= limits::OPEN_MAX {
			return Err(errno!(EMFILE));
		}
		// Find the beginning index for the search of the ID
		let start = self.0.binary_search_by(|fd| fd.get_id().cmp(&min));
		let Ok(start) = start else {
			return Ok(min);
		};
		// Search for an unused ID
		let mut prev = min;
		for fd in &self.0[start..] {
			let fd = fd.get_id();
			if fd - prev > 1 {
				return Ok(prev + 1);
			}
			prev = fd;
		}
		// No hole found, place the new FD at the end
		let id = self.0.last().map(|fd| fd.get_id() + 1).unwrap_or(0);
		let id = max(id, min);
		if id < limits::OPEN_MAX {
			Ok(id)
		} else {
			Err(errno!(EMFILE))
		}
	}

	/// Creates a file descriptor and returns a pointer to it with its ID.
	///
	/// Arguments:
	/// - `flags` are the file descriptor's flags
	/// - `open_file` is the file associated with the file descriptor
	pub fn create_fd(&mut self, flags: i32, open_file: OpenFile) -> EResult<&FileDescriptor> {
		// Create the file descriptor
		let id = self.get_available_fd(None)?;
		let fd = FileDescriptor::new(id, flags, open_file)?;
		// Insert the file descriptor
		let i = self
			.0
			.binary_search_by(|fd| fd.get_id().cmp(&id))
			.unwrap_err();
		self.0.insert(i, fd)?;
		Ok(&self.0[i])
	}

	/// Returns an immutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor does not exist, the function returns [`EBADF`].
	pub fn get_fd(&self, id: c_int) -> EResult<&FileDescriptor> {
		let id: u32 = id.try_into().map_err(|_| errno!(EBADF))?;
		let result = self.0.binary_search_by(|fd| fd.get_id().cmp(&id));
		let Ok(index) = result else {
			return Err(errno!(EBADF));
		};
		Ok(&self.0[index])
	}

	/// Returns a mutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor does not exist, the function returns [`EBADF`].
	pub fn get_fd_mut(&mut self, id: c_int) -> EResult<&mut FileDescriptor> {
		let id: u32 = id.try_into().map_err(|_| errno!(EBADF))?;
		let result = self.0.binary_search_by(|fd| fd.get_id().cmp(&id));
		let Ok(index) = result else {
			return Err(errno!(EBADF));
		};
		Ok(&mut self.0[index])
	}

	/// Duplicates the file descriptor with id `id`.
	///
	/// Arguments:
	/// - `constraint` is the constraint the new file descriptor ID will follow.
	/// - `cloexec` tells whether the new file descriptor has the `FD_CLOEXEC` flag enabled.
	///
	/// The function returns a pointer to the file descriptor with its ID.
	pub fn duplicate_fd(
		&mut self,
		id: c_int,
		constraint: NewFDConstraint,
		cloexec: bool,
	) -> EResult<&FileDescriptor> {
		// The ID of the new FD
		let new_id = match constraint {
			NewFDConstraint::None => self.get_available_fd(None)?,
			NewFDConstraint::Fixed(id) => {
				if id >= limits::OPEN_MAX {
					return Err(errno!(EMFILE));
				}
				id
			}
			NewFDConstraint::Min(min) => self.get_available_fd(Some(min))?,
		};
		// The old FD
		let old_fd = self.get_fd(id)?;
		// Create the new FD
		let mut new_fd = old_fd.duplicate(new_id);
		let flags = if cloexec { FD_CLOEXEC } else { 0 };
		new_fd.flags = flags;
		// Insert the FD
		let index = self.0.binary_search_by(|fd| fd.get_id().cmp(&new_id));
		let index = match index {
			Ok(i) => {
				self.0[i] = new_fd;
				i
			}
			Err(i) => {
				self.0.insert(i, new_fd)?;
				i
			}
		};
		Ok(&self.0[index])
	}

	/// Duplicates the whole file descriptors table.
	///
	/// `cloexec` specifies whether the cloexec flag must be taken into account. This is the case
	/// when executing a program.
	pub fn duplicate(&self, cloexec: bool) -> EResult<Self> {
		let fds = self
			.0
			.iter()
			.filter(|fd| {
				// cloexec implies the FD's cloexec flag must be clear
				!cloexec || fd.flags & FD_CLOEXEC == 0
			})
			.cloned()
			.collect::<CollectResult<Vec<_>>>()
			.0?;
		Ok(Self(fds))
	}

	/// Closes the file descriptor with the ID `id`.
	///
	/// The function returns an Err if the file descriptor doesn't exist.
	pub fn close_fd(&mut self, id: c_int) -> EResult<()> {
		let id: u32 = id.try_into().map_err(|_| errno!(EBADF))?;
		let result = self.0.binary_search_by(|fd| fd.get_id().cmp(&id));
		let Ok(index) = result else {
			return Err(errno!(EBADF));
		};
		let fd = self.0.remove(index);
		fd.close()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::file::{File, FileLocation, Stat};

	/// Creates a dummy open file for testing purpose.
	fn dummy_open_file() -> OpenFile {
		let file = File::new(FileLocation::dummy(), None, Stat::default());
		OpenFile::new(Arc::new(Mutex::new(file)).unwrap(), None, 0).unwrap()
	}

	#[test_case]
	fn fd_create0() {
		let mut fds = FileDescriptorTable::default();
		let fd = fds.create_fd(0, dummy_open_file()).unwrap().get_id();
		assert_eq!(fd, 0);
	}

	#[test_case]
	fn fd_create1() {
		let mut fds = FileDescriptorTable::default();
		let fd = fds.create_fd(0, dummy_open_file()).unwrap().get_id();
		assert_eq!(fd, 0);
		let fd = fds.create_fd(0, dummy_open_file()).unwrap().get_id();
		assert_eq!(fd, 1);
	}

	#[test_case]
	fn fd_dup() {
		let mut fds = FileDescriptorTable::default();
		let fd = fds.create_fd(0, dummy_open_file()).unwrap().get_id();
		assert_eq!(fd, 0);
		let fd0 = fds
			.duplicate_fd(0, NewFDConstraint::None, false)
			.unwrap()
			.get_id();
		assert_ne!(fd0, 0);
		let fd1 = fds
			.duplicate_fd(0, NewFDConstraint::Fixed(16), false)
			.unwrap()
			.get_id();
		assert_eq!(fd1, 16);
		let fd2 = fds
			.duplicate_fd(0, NewFDConstraint::Min(8), false)
			.unwrap()
			.get_id();
		assert!(fd2 >= 8);
		let fd3 = fds
			.duplicate_fd(0, NewFDConstraint::Min(8), false)
			.unwrap()
			.get_id();
		assert!(fd3 >= 8);
		assert_ne!(fd3, fd2);
	}
}
