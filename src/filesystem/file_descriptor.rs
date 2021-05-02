/// TODO doc

use core::ptr::NonNull;
use crate::filesystem::File;
use crate::util::FailableClone;

/// Structure representing a file descriptor.
#[derive(Clone)]
pub struct FileDescriptor {
	/// The ID of the file descriptor.
	id: u32,
	/// A pointer to the file the descriptor is linked to.
	file: NonNull::<File>, // TODO Fix: if the file is removed, this will be deleted
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	pub fn new(id: u32, file: &mut File) -> Self {
		Self {
			id: id,
			file: NonNull::new(file as *mut File).unwrap(),
		}
	}

	/// Returns the id of the file descriptor.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns a mutable reference to the file associated to the descriptor.
	pub fn get_file(&mut self) -> &mut File {
		unsafe {
			self.file.as_mut()
		}
	}

	// TODO
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		// TODO Close the fd
	}
}
