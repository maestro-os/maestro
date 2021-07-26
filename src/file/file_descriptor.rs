//! A file descriptor is a sort of pointer to a file, allowing a process to manipulate the
//! filesystem through system calls.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::pipe::Pipe;
use crate::limits;
use crate::util::FailableClone;
use crate::util::lock::mutex::Mutex;
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
	let mut guard = mutex.lock(true);

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
	let mut guard = mutex.lock(true);
	*guard.get_mut() -= 1;
}

/// Enumeration of every possible targets for a file descriptor.
#[derive(Clone)]
pub enum FDTarget {
	/// The file descriptor points to a file.
	File(SharedPtr<File>),
	/// The file descriptor points to a pipe.
	FileDescriptor(SharedPtr<Pipe>),
}

/// Structure representing a file descriptor.
#[derive(Clone)]
pub struct FileDescriptor {
	/// The ID of the file descriptor.
	id: u32,
	/// The file descriptor's flags.
	flags: i32,
	/// A pointer to the file the descriptor is linked to.
	target: FDTarget,

	/// The current offset in the file.
	curr_off: u64,
}

impl FileDescriptor {
	/// Creates a new file descriptor.
	pub fn new(id: u32, flags: i32, target: FDTarget) -> Result<Self, Errno> {
		increment_total()?;

		Ok(Self {
			id,
			flags,
			target,

			curr_off: 0,
		})
	}

	/// Returns the id of the file descriptor.
	pub fn get_id(&self) -> u32 {
		self.id
	}

	/// Returns the file descriptor's flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Returns a mutable reference to the descriptor's target.
	pub fn get_target(&self) -> &FDTarget {
		&self.target
	}

	/// Returns an immutable reference to the descriptor's target.
	pub fn get_target_mut(&mut self) -> &mut FDTarget {
		&mut self.target
	}

	/// Returns the size of the file's content in bytes.
	pub fn get_file_size(&self) -> u64 {
		if let FDTarget::File(f) = &self.target {
			f.get_mut().lock(true).get().get_size()
		} else {
			0
		}
	}

	/// Returns the current offset in the file.
	pub fn get_offset(&self) -> u64 {
		self.curr_off
	}

	/// Sets the current offset in the file.
	pub fn set_offset(&mut self, off: u64) {
		self.curr_off = off;
	}

	/// Reads data from the file.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
		let len = match &mut self.target {
			FDTarget::File(f) => {
				let mut guard = f.lock(true);
				guard.get_mut().read(self.curr_off as usize, buf)?
			},

			FDTarget::FileDescriptor(p) => {
				let mut guard = p.lock(true);
				guard.get_mut().read(buf)
			}
		};

		self.curr_off += len as u64;
		Ok(len)
	}

	/// Writes data to the file.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> Result<usize, Errno> {
		let len = match &mut self.target {
			FDTarget::File(f) => {
				let mut guard = f.lock(true);
				guard.get_mut().write(self.curr_off as usize, buf)?
			},

			FDTarget::FileDescriptor(p) => {
				let mut guard = p.lock(true);
				guard.get_mut().write(buf)
			}
		};

		self.curr_off += len as u64;
		Ok(len)
	}
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		decrement_total();
	}
}
