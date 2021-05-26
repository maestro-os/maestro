//! A file descriptor is a sort of pointer to a file, allowing a process to manipulate the
//! filesystem through system calls.

use crate::file::File;
use crate::util::FailableClone;
use crate::util::ptr::SharedPtr;

/// Structure representing a file descriptor.
#[derive(Clone)]
pub struct FileDescriptor {
	/// The ID of the file descriptor.
	id: u32,
	/// A pointer to the file the descriptor is linked to.
	file: SharedPtr<File>,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	pub fn new(id: u32, file: SharedPtr<File>) -> Self {
		Self {
			id,
			file,
		}
	}

	/// Returns the id of the file descriptor.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns a mutable reference to the file associated to the descriptor.
	pub fn get_file(&mut self) -> &SharedPtr<File> {
		&self.file
	}

	// TODO
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		// TODO Close the fd
	}
}
