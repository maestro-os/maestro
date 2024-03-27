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
use core::cmp::max;
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
#[derive(Clone)]
pub struct FileDescriptor {
	/// The FD's id
	id: u32,
	/// The FD's flags
	flags: i32,

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

	/// Returns the file descriptor's flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Sets the file descriptor's flags.
	pub fn set_flags(&mut self, flags: i32) {
		self.flags = flags;
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

/// A table of file descriptors.
pub struct FileDescriptorTable {
	// TODO use a BTreeMap or BTreeSet instead?
	/// The list of file descriptors.
	fds: Vec<FileDescriptor>,
}

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

		let start = match self.fds.binary_search_by(|fd| fd.get_id().cmp(&min)) {
			Ok(i) => i,
			Err(_) => return Ok(min),
		};

		let mut prev = min;
		for fd in &self.fds[start..] {
			let fd = fd.get_id();
			if fd - prev > 1 {
				return Ok(prev + 1);
			}
			prev = fd;
		}

		// unwrap cannot fail because
		let id = self.fds.last().map(|fd| fd.get_id() + 1).unwrap();
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
		let id = self.get_available_fd(None)?;
		let i = self
			.fds
			.binary_search_by(|fd| fd.get_id().cmp(&id))
			.unwrap_err();

		let fd = FileDescriptor::new(id, flags, open_file)?;
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
		let old_fd = self.get_fd(id).ok_or_else(|| errno!(EBADF))?;

		// Create the new FD
		let mut new_fd = old_fd.duplicate(new_id);
		let flags = if cloexec { FD_CLOEXEC } else { 0 };
		new_fd.set_flags(flags);

		// Insert the FD
		let index = self.fds.binary_search_by(|fd| fd.get_id().cmp(&new_id));
		let index = match index {
			Ok(i) => {
				self.fds[i] = new_fd;
				i
			}
			Err(i) => {
				self.fds.insert(i, new_fd)?;
				i
			}
		};

		Ok(&self.fds[index])
	}

	/// Duplicates the whole file descriptors table.
	///
	/// `cloexec` specifies whether the cloexec flag must be taken into account. This is the case
	/// when executing a program.
	pub fn duplicate(&self, cloexec: bool) -> EResult<Self> {
		let fds = self
			.fds
			.iter()
			.filter(|fd| {
				// cloexec implies fd's cloexec flag must be clear
				!cloexec || fd.get_flags() & FD_CLOEXEC == 0
			})
			.cloned()
			.collect::<CollectResult<Vec<_>>>()
			.0?;
		Ok(Self {
			fds,
		})
	}

	/// Closes the file descriptor with the ID `id`.
	///
	/// The function returns an Err if the file descriptor doesn't exist.
	pub fn close_fd(&mut self, id: u32) -> EResult<()> {
		let result = self.fds.binary_search_by(|fd| fd.get_id().cmp(&id));
		if let Ok(index) = result {
			let fd = self.fds.remove(index);
			fd.close()
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::file::{File, FileLocation};
	use utils::{lock::Mutex, ptr::arc::Arc};

	/// Creates a dummy open file for testing purpose.
	fn dummy_open_file() -> OpenFile {
		const DUMMY_LOCATION: FileLocation = FileLocation::Virtual {
			id: 0,
		};
		let file = File::new(DUMMY_LOCATION, 0, 0, 0, 0).unwrap();
		OpenFile::new(Arc::new(Mutex::new(file)).unwrap(), 0).unwrap()
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
