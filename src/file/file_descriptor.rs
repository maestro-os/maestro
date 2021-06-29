//! A file descriptor is a sort of pointer to a file, allowing a process to manipulate the
//! filesystem through system calls.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::limits;
use crate::util::FailableClone;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::lock::mutex::TMutex;
use crate::util::ptr::SharedPtr;

/// The total number of file descriptors open system-wide.
static mut TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
/// If the maximum amount of file descriptors is reached, the function does nothing and returns an
/// error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let mutex = unsafe { // Safe because using Mutex
		&mut TOTAL_FD
	};
	let mut guard = MutexGuard::new(mutex);

	// TODO Use the correct constant
	if *guard.get() >= limits::OPEN_MAX {
		return Err(errno::ENFILE);
	}
	*guard.get_mut() += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	let mutex = unsafe { // Safe because using Mutex
		&mut TOTAL_FD
	};
	let mut guard = MutexGuard::new(mutex);
	*guard.get_mut() -= 1;
}

/// Structure representing a file descriptor.
#[derive(Clone)]
pub struct FileDescriptor {
	/// The ID of the file descriptor.
	id: u32,
	/// A pointer to the file the descriptor is linked to.
	file: SharedPtr<File>,

	/// The current offset in the file.
	curr_off: u64,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	pub fn new(id: u32, file: SharedPtr<File>) -> Result<Self, Errno> {
		increment_total()?;

		Ok(Self {
			id,
			file,

			curr_off: 0,
		})
	}

	/// Returns the id of the file descriptor.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns a mutable reference to the file associated to the descriptor.
	pub fn get_file(&self) -> &SharedPtr<File> {
		&self.file
	}

	/// Returns an immutable reference to the file associated to the descriptor.
	pub fn get_file_mut(&mut self) -> &mut SharedPtr<File> {
		&mut self.file
	}

	/// Returns the size of the file's content in bytes.
	pub fn get_file_size(&self) -> u64 {
		self.file.get_mut().lock().get().get_size()
	}

	/// Returns the current offset in the file.
	pub fn get_offset(&self) -> u64 {
		self.curr_off
	}

	/// Sets the current offset in the file.
	pub fn set_offset(&mut self, off: u64) {
		self.curr_off = off;
	}
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		decrement_total();
	}
}
