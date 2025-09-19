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

//! File descriptors implementation.
//!
//! A file descriptor is an ID held by a process pointing to an entry in the
//! open file description table.

use crate::{file::File, process::Process};
use core::{cmp::max, ffi::c_int, mem};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{AllocResult, CollectResult, EResult},
	limits::OPEN_MAX,
	ptr::arc::Arc,
};

/// File descriptor flag: If set, the file descriptor is closed on successful
/// call to `execve`.
pub const FD_CLOEXEC: i32 = 1;

/// Constraint on a new file descriptor ID.
#[derive(Debug)]
pub enum NewFDConstraint {
	/// No constraint
	None,
	/// The new file descriptor must have given fixed value
	Fixed(c_int),
	/// The new file descriptor must have at least the given value
	Min(u32),
}

/// A file descriptor, pointing to a [`File`].
#[derive(Clone, Debug)]
pub struct FileDescriptor {
	/// The file descriptor's flags.
	pub flags: i32,
	/// The open file description associated with the file descriptor.
	file: Arc<File>,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	///
	/// If no open file description is associated with the given location, the function creates
	/// one.
	///
	/// Arguments:
	/// - `flags` is the set of flags associated with the file descriptor
	/// - `location` is the location of the open file the file descriptor points to
	pub fn new(flags: i32, file: Arc<File>) -> EResult<Self> {
		Ok(Self {
			flags,
			file,
		})
	}

	/// Returns the open file associated with the descriptor.
	pub fn get_file(&self) -> &Arc<File> {
		&self.file
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
		let Some(file) = Arc::into_inner(self.file) else {
			return Ok(());
		};
		file.close()
	}
}

/// A table of file descriptors.
#[derive(Default)]
pub struct FileDescriptorTable(Vec<Option<FileDescriptor>>);

impl FileDescriptorTable {
	/// Returns the available file descriptor with the lowest ID.
	///
	/// If no ID is available, the function returns an error.
	///
	/// `min` is the minimum value for the file descriptor to be returned.
	fn get_available_fd(&self, min: Option<u32>) -> EResult<u32> {
		let min = min.unwrap_or(0) as usize;
		// Find a hole in the table
		let fd = if min < self.0.len() {
			self.0[min..]
				.iter()
				.enumerate()
				.find(|(_, fd)| fd.is_none())
				.map(|(i, _)| (min + i) as u32)
		} else {
			None
		};
		match fd {
			Some(fd) => Ok(fd),
			// No hole found, place the new FD at the end
			None => {
				let id = max(self.0.len(), min) as u32;
				if id < OPEN_MAX {
					Ok(id)
				} else {
					Err(errno!(EMFILE))
				}
			}
		}
	}

	/// Extends the file descriptor table if necessary so that it can fit the given ID.
	///
	/// If the table is already large enough, this function is a no-op.
	fn extend(&mut self, id: u32) -> AllocResult<()> {
		let id = id as usize;
		// The ID fits. Do nothing
		if id < self.0.len() {
			return Ok(());
		}
		self.0.resize(id + 1, None)
	}

	/// Creates a file descriptor.
	///
	/// Arguments:
	/// - `flags` are the file descriptor's flags
	/// - `file` is the file associated with the file descriptor
	///
	/// The function returns the ID of the new file descriptor alongside a reference to it.
	pub fn create_fd(&mut self, flags: i32, file: Arc<File>) -> EResult<(u32, &FileDescriptor)> {
		let id = self.get_available_fd(None)?;
		let fd = FileDescriptor::new(flags, file)?;
		// Insert the FD
		self.extend(id)?;
		let fd = self.0[id as usize].insert(fd);
		Ok((id, fd))
	}

	/// Creates a pair of file descriptors. The `flags` field is set to zero for both.
	///
	/// This function is a helper for system calls that create pipe or pipe-like objects. It allows
	/// to ensure the first file descriptor is not created if the creation of the second fails.
	///
	/// Arguments:
	/// - `file0` is the file associated with the first file descriptor
	/// - `file1` is the file associated with the second file descriptor
	///
	/// The function returns the IDs of the new file descriptors.
	pub fn create_fd_pair(&mut self, file0: Arc<File>, file1: Arc<File>) -> EResult<(u32, u32)> {
		let id0 = self.get_available_fd(None)?;
		// Add a constraint to avoid using twice the same ID
		let id1 = self.get_available_fd(Some(id0 + 1))?;
		let fd0 = FileDescriptor::new(0, file0)?;
		let fd1 = FileDescriptor::new(0, file1)?;
		// Insert the FDs
		self.extend(id1)?; // `id1` is always larger than `id0`
		self.0[id0 as usize] = Some(fd0);
		self.0[id1 as usize] = Some(fd1);
		Ok((id0, id1))
	}

	/// Returns an immutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor does not exist, the function returns [`errno::EBADF`].
	pub fn get_fd(&self, id: c_int) -> EResult<&FileDescriptor> {
		let id: usize = id.try_into().map_err(|_| errno!(EBADF))?;
		self.0
			.get(id)
			.and_then(Option::as_ref)
			.ok_or_else(|| errno!(EBADF))
	}

	/// Returns a mutable reference to the file descriptor with ID `id`.
	///
	/// If the file descriptor does not exist, the function returns [`errno::EBADF`].
	pub fn get_fd_mut(&mut self, id: c_int) -> EResult<&mut FileDescriptor> {
		let id: usize = id.try_into().map_err(|_| errno!(EBADF))?;
		self.0
			.get_mut(id)
			.and_then(Option::as_mut)
			.ok_or_else(|| errno!(EBADF))
	}

	/// Duplicates the file descriptor with id `id`.
	///
	/// Arguments:
	/// - `constraint` is the constraint the new file descriptor ID will follow.
	/// - `cloexec` tells whether the new file descriptor has the `FD_CLOEXEC` flag enabled.
	///
	/// The function returns the ID of the new file descriptor alongside a reference to it.
	pub fn duplicate_fd(
		&mut self,
		id: c_int,
		constraint: NewFDConstraint,
		cloexec: bool,
	) -> EResult<(u32, &FileDescriptor)> {
		// The ID of the new FD
		let new_id = match constraint {
			NewFDConstraint::None => self.get_available_fd(None)?,
			NewFDConstraint::Fixed(id) => {
				let id: u32 = id.try_into().map_err(|_| errno!(EBADF))?;
				if id >= OPEN_MAX {
					return Err(errno!(EMFILE));
				}
				id
			}
			NewFDConstraint::Min(min) => self.get_available_fd(Some(min))?,
		};
		// The old FD
		let old_fd = self.get_fd(id)?;
		// Create the new FD
		let mut new_fd = old_fd.clone();
		let flags = if cloexec { FD_CLOEXEC } else { 0 };
		new_fd.flags = flags;
		// Make sure the table is large enough
		self.extend(new_id)?;
		// If there was a file descriptor in the slot, close it
		let slot = &mut self.0[new_id as usize];
		if let Some(prev) = slot.take() {
			let _ = prev.close();
		}
		// Insert the FD
		let new_fd = slot.insert(new_fd);
		Ok((new_id, new_fd))
	}

	/// Duplicates the whole file descriptors table.
	///
	/// `cloexec` specifies whether the cloexec flag must be taken into account. This is the case
	/// when executing a program.
	pub fn duplicate(&self, cloexec: bool) -> EResult<Self> {
		let fds = self
			.0
			.iter()
			.cloned()
			.map(|fd| {
				fd.filter(|fd| {
					// cloexec implies the FD's cloexec flag must be clear
					!cloexec || fd.flags & FD_CLOEXEC == 0
				})
			})
			.collect::<CollectResult<Vec<_>>>()
			.0?;
		Ok(Self(fds))
	}

	/// Closes the file descriptor with the ID `id`.
	///
	/// If the file descriptor does not exist, the function returns [`errno::EBADF`].
	pub fn close_fd(&mut self, id: c_int) -> EResult<()> {
		let id: usize = id.try_into().map_err(|_| errno!(EBADF))?;
		let fd = self.0.get_mut(id).ok_or_else(|| errno!(EBADF))?;
		// Remove FD from table
		let Some(fd) = fd.take() else {
			return Err(errno!(EBADF));
		};
		// Shrink the table if necessary
		let new_len = self
			.0
			.iter()
			.enumerate()
			.rfind(|(_, fd)| fd.is_some())
			.map(|(i, _)| i + 1)
			.unwrap_or(0);
		self.0.truncate(new_len);
		// Close FD
		fd.close()
	}
}

impl Drop for FileDescriptorTable {
	fn drop(&mut self) {
		let fds = mem::take(&mut self.0);
		for fd in fds.into_iter().flatten() {
			let _ = fd.close();
		}
	}
}

/// Returns a reference to the file associated with the file descriptor `fd` on the current
/// process.
///
/// If the file descriptor does not exist, the function returns [`errno::EBADF`].
pub fn fd_to_file(fd: c_int) -> EResult<Arc<File>> {
	let file = Process::current()
		.file_descriptors()
		.lock()
		.get_fd(fd)?
		.get_file()
		.clone();
	Ok(file)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::file::{File, fs::FileOps};

	/// Dummy node ops for testing purpose.
	#[derive(Debug)]
	struct Dummy;

	impl FileOps for Dummy {}

	/// Creates a dummy open file for testing purpose.
	fn dummy_file() -> Arc<File> {
		File::open_floating(Arc::new(Dummy).unwrap(), 0).unwrap()
	}

	#[test_case]
	fn fd_create0() {
		let mut fds = FileDescriptorTable::default();
		let (id, _) = fds.create_fd(0, dummy_file()).unwrap();
		assert_eq!(id, 0);
	}

	#[test_case]
	fn fd_create1() {
		let mut fds = FileDescriptorTable::default();
		let (id, _) = fds.create_fd(0, dummy_file()).unwrap();
		assert_eq!(id, 0);
		let (id, _) = fds.create_fd(0, dummy_file()).unwrap();
		assert_eq!(id, 1);
	}

	#[test_case]
	fn fd_dup() {
		let mut fds = FileDescriptorTable::default();
		let (id, _) = fds.create_fd(0, dummy_file()).unwrap();
		assert_eq!(id, 0);
		let (id0, _) = fds.duplicate_fd(0, NewFDConstraint::None, false).unwrap();
		assert_ne!(id0, 0);
		let (id1, _) = fds
			.duplicate_fd(0, NewFDConstraint::Fixed(16), false)
			.unwrap();
		assert_eq!(id1, 16);
		let (id2, _) = fds.duplicate_fd(0, NewFDConstraint::Min(8), false).unwrap();
		assert!(id2 >= 8);
		let (id3, _) = fds.duplicate_fd(0, NewFDConstraint::Min(8), false).unwrap();
		assert!(id3 >= 8);
		assert_ne!(id3, id2);
	}
}
