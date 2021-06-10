//! A file descriptor is a sort of pointer to a file, allowing a process to manipulate the
//! filesystem through system calls.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::limits;
use crate::util::FailableClone;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::ptr::SharedPtr;

/// The total number of file descriptors open system-wide.
static mut TOTAL_FD: Mutex<usize> = Mutex::new(0);

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
	pub fn new(id: u32, file: SharedPtr<File>) -> Result<Self, Errno> {
		{
			let mutex = unsafe { // Safe because using Mutex
				&mut TOTAL_FD
			};
			let mut guard = MutexGuard::new(mutex);
			if *guard.get() >= limits::OPEN_MAX {
				return Err(errno::ENFILE);
			}
			*guard.get_mut() += 1;
		}

		Ok(Self {
			id,
			file,
		})
	}

	/// Returns the id of the file descriptor.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns a mutable reference to the file associated to the descriptor.
	pub fn get_file(&mut self) -> &SharedPtr<File> {
		&self.file
	}
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		{
			let mutex = unsafe { // Safe because using Mutex
				&mut TOTAL_FD
			};
			let mut guard = MutexGuard::new(mutex);
			*guard.get_mut() -= 1;
		}

		// TODO Close the fd
		todo!();
	}
}
