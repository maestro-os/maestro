//! A file descriptor is a sort of pointer to a file, allowing a process to manipulate the
//! filesystem through system calls.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::pipe::Pipe;
use crate::file::socket::SocketSide;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// Read only.
pub const O_RDONLY: i32 =    0b0000000000000001;
/// Write only.
pub const O_WRONLY: i32 =    0b0000000000000010;
/// Read and write.
pub const O_RDWR: i32 =      0b0000000000000011;
/// At each write operations on the file descriptor, the cursor is placed at the end of the file so
/// the data is appended.
pub const O_APPEND: i32 =    0b0000000000000100;
/// Generates a SIGIO when input or output becomes possible on the file descriptor.
pub const O_ASYNC: i32 =     0b0000000000001000;
/// Close-on-exec.
pub const O_CLOEXEC: i32 =   0b0000000000010000;
/// If the file doesn't exist, create it.
pub const O_CREAT: i32 =     0b0000000000100000;
/// Disables caching data.
pub const O_DIRECT: i32 =    0b0000000001000000;
/// If pathname is not a directory, cause the open to fail.
pub const O_DIRECTORY: i32 = 0b0000000010000000;
/// Ensure the file is created (when used with O_CREAT). If not, the call fails.
pub const O_EXCL: i32 =      0b0000000100000000;
/// Allows openning large files (more than 2^32 bytes).
pub const O_LARGEFILE: i32 = 0b0000001000000000;
/// Don't update file access time.
pub const O_NOATIME: i32 =   0b0000010000000000;
/// If refering to a tty, it will not become the process's controlling tty.
pub const O_NOCTTY: i32 =    0b0000100000000000;
/// Tells `open` not to follow symbolic links.
pub const O_NOFOLLOW: i32 =  0b0001000000000000;
/// I/O is non blocking.
pub const O_NONBLOCK: i32 =  0b0010000000000000;
/// When using `write`, the data has been transfered to the hardware before returning.
pub const O_SYNC: i32 =      0b0100000000000000;
/// If the file already exists, truncate it to length zero.
pub const O_TRUNC: i32 =     0b1000000000000000;

/// The maximum number of file descriptors that can be open system-wide at once.
const TOTAL_MAX_FD: usize = 4294967295;

/// The total number of file descriptors open system-wide.
static TOTAL_FD: Mutex<usize> = Mutex::new(0);

/// Increments the total number of file descriptors open system-wide.
/// If the maximum amount of file descriptors is reached, the function does nothing and returns an
/// error with the appropriate errno.
fn increment_total() -> Result<(), Errno> {
	let mut guard = TOTAL_FD.lock();

	if *guard.get() >= TOTAL_MAX_FD {
		return Err(errno::ENFILE);
	}
	*guard.get_mut() += 1;

	Ok(())
}

/// Decrements the total number of file descriptors open system-wide.
fn decrement_total() {
	let mut guard = TOTAL_FD.lock();
	*guard.get_mut() -= 1;
}

/// Enumeration of every possible targets for a file descriptor.
#[derive(Clone)]
pub enum FDTarget {
	/// The file descriptor points to a file.
	File(SharedPtr<File>),
	/// The file descriptor points to a pipe.
	Pipe(SharedPtr<Pipe>),
	/// The file descriptor points to a socket.
	Socket(SharedPtr<SocketSide>),
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

	/// Sets the file descriptor's flags.
	pub fn set_flags(&mut self, flags: i32) {
		self.flags = flags;
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
			f.get_mut().lock().get().get_size()
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

	/// Returns the length of the file the descriptor points to.
	pub fn get_len(&mut self) -> u64 {
		match &mut self.target {
			FDTarget::File(f) => {
				let guard = f.lock();
				guard.get().get_size()
			},

			FDTarget::Pipe(_p) => {
				// TODO Get the fd the pipe points to, then make a recursive call
				todo!();
			}

			FDTarget::Socket(_s) => {
				// TODO
				todo!();
			}
		}
	}

	/// Reads data from the file.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, Errno> {
		if self.flags & O_RDONLY == 0 {
			return Err(errno::EINVAL);
		}

		let len = match &mut self.target {
			FDTarget::File(f) => {
				let mut guard = f.lock();
				guard.get_mut().read(self.curr_off, buf)?
			},

			FDTarget::Pipe(p) => {
				let mut guard = p.lock();
				guard.get_mut().read(buf) as _
			}

			FDTarget::Socket(s) => {
				let mut guard = s.lock();
				guard.get_mut().read(buf) as _
			},
		};

		self.curr_off += len as u64;
		Ok(len as _)
	}

	/// Writes data to the file.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> Result<usize, Errno> {
		if self.flags & O_WRONLY == 0 {
			return Err(errno::EINVAL);
		}

		let len = match &mut self.target {
			FDTarget::File(f) => {
				let mut guard = f.lock();
				guard.get_mut().write(self.curr_off, buf)?
			},

			FDTarget::Pipe(p) => {
				let mut guard = p.lock();
				guard.get_mut().write(buf) as _
			}

			FDTarget::Socket(s) => {
				let mut guard = s.lock();
				guard.get_mut().write(buf) as _
			},
		};

		self.curr_off += len as u64;
		Ok(len as _)
	}

	/// Performs an ioctl operation on the file descriptor.
	pub fn ioctl(&mut self, request: u32, argp: *const c_void) -> Result<u32, Errno> {
		match &mut self.target {
			FDTarget::File(f) => {
				let mut guard = f.lock();
				guard.get_mut().ioctl(request, argp)
			},

			FDTarget::Pipe(_) => {
				// TODO Get corresponding fd
				todo!();
			}

			FDTarget::Socket(_) => Err(errno::EINVAL),
		}
	}
}

crate::failable_clone_impl!(FileDescriptor);

impl Drop for FileDescriptor {
	fn drop(&mut self) {
		decrement_total();
	}
}
