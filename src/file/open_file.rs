//! An open file description is a structure pointing to a file, allowing to perform operations on
//! it. It is pointed to by file descriptors.

use core::cmp::min;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::pipe::PipeBuffer;
use crate::file::socket::SocketSide;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::syscall::ioctl;
use crate::time::unit::TimestampScale;
use crate::time;
use crate::types::c_int;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// Read only.
pub const O_RDONLY: i32 = 0b00000000000000000000000000000000;
/// Write only.
pub const O_WRONLY: i32 = 0b00000000000000000000000000000001;
/// Read and write.
pub const O_RDWR: i32 = 0b00000000000000000000000000000010;
/// At each write operations, the cursor is placed at the end of the file so the data is appended.
pub const O_APPEND: i32 = 0b00000000000000000000010000000000;
/// Generates a SIGIO when input or output becomes possible on the file.
pub const O_ASYNC: i32 = 0b00000000000000000010000000000000;
/// Close-on-exec.
pub const O_CLOEXEC: i32 = 0b00000000000010000000000000000000;
/// If the file doesn't exist, create it.
pub const O_CREAT: i32 = 0b00000000000000000000000001000000;
/// Disables caching data.
pub const O_DIRECT: i32 = 0b00000000000000000100000000000000;
/// If pathname is not a directory, cause the open to fail.
pub const O_DIRECTORY: i32 = 0b00000000000000010000000000000000;
/// Ensure the file is created (when used with O_CREAT). If not, the call fails.
pub const O_EXCL: i32 = 0b00000000000000000000000010000000;
/// Allows openning large files (more than 2^32 bytes).
pub const O_LARGEFILE: i32 = 0b00000000000000001000000000000000;
/// Don't update file access time.
pub const O_NOATIME: i32 = 0b00000000000001000000000000000000;
/// If refering to a tty, it will not become the process's controlling tty.
pub const O_NOCTTY: i32 = 0b00000000000000000000000100000000;
/// Tells `open` not to follow symbolic links.
pub const O_NOFOLLOW: i32 = 0b00000000000000100000000000000000;
/// I/O is non blocking.
pub const O_NONBLOCK: i32 = 0b00000000000000000000100000000000;
/// When using `write`, the data has been transfered to the hardware before returning.
pub const O_SYNC: i32 = 0b00000000000100000001000000000000;
/// If the file already exists, truncate it to length zero.
pub const O_TRUNC: i32 = 0b00000000000000000000001000000000;

/// Enumeration of every possible targets for an open file.
#[derive(Clone, Debug)]
pub enum FDTarget {
	/// Points to a file.
	File(SharedPtr<File>),
	/// Points to a pipe.
	Pipe(SharedPtr<PipeBuffer>),
	/// Points to a socket.
	Socket(SharedPtr<SocketSide>),
}

impl FDTarget {
	/// Returns the file associated with the target.
	pub fn get_file(&self) -> Result<SharedPtr<File>, Errno> {
		match self {
			Self::File(f) => Ok(f.clone()),

			Self::Pipe(_p) => {
				// TODO
				let file = File::new(
					String::from(b"TODO")?,
					0,
					0,
					0o777,
					FileLocation {
						mountpoint_id: None,

						inode: 0,
					},
					FileContent::Fifo,
				)?;

				SharedPtr::new(file)
			}

			Self::Socket(_s) => {
				// TODO
				todo!();
			}
		}
	}
}

/// An open file description. This structure is pointed to by file descriptors and point to files.
/// They exist to ensure several file descriptors can share the same open file.
#[derive(Debug)]
pub struct OpenFile {
	/// The open file description's flags.
	flags: i32,
	/// A pointer to the target file.
	target: FDTarget,

	/// The current offset in the file.
	/// If pointing to a directory, this is the offset in directory entries.
	curr_off: u64,
}

impl OpenFile {
	/// Creates a new open file description.
	pub fn new(flags: i32, target: FDTarget) -> Result<Self, Errno> {
		let s = Self {
			flags,
			target,

			curr_off: 0,
		};

		// Update references count
		match &s.target {
			FDTarget::File(file) => file.lock().get_mut().increment_open(s.can_write())?,
			FDTarget::Pipe(pipe) => pipe.lock().get_mut().increment_open(s.can_write()),

			_ => {}
		}

		Ok(s)
	}

	/// Returns the file flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Sets the open file flags.
	/// File access mode (O_RDONLY, O_WRONLY, O_RDWR) and file creation flags (O_CREAT, O_EXCL,
	/// O_NOCTTY, O_TRUNC) are ignored.
	pub fn set_flags(&mut self, flags: i32) {
		let ignored_flags = O_RDONLY | O_WRONLY | O_RDWR | O_CREAT | O_EXCL | O_NOCTTY | O_TRUNC;
		self.flags = (self.flags & ignored_flags) | (flags & !ignored_flags);
	}

	/// Tells whether the open file can be read from.
	pub fn can_read(&self) -> bool {
		matches!(self.flags & 0b11, O_RDONLY | O_RDWR)
	}

	/// Tells whether the open file can be written to.
	pub fn can_write(&self) -> bool {
		matches!(self.flags & 0b11, O_WRONLY | O_RDWR)
	}

	/// Returns a mutable reference to the target.
	pub fn get_target(&self) -> &FDTarget {
		&self.target
	}

	/// Returns an immutable reference to the target.
	pub fn get_target_mut(&mut self) -> &mut FDTarget {
		&mut self.target
	}

	/// Returns the current offset in the file.
	pub fn get_offset(&self) -> u64 {
		self.curr_off
	}

	/// Sets the current offset in the file.
	pub fn set_offset(&mut self, off: u64) {
		self.curr_off = off;
	}

	/// Performs an ioctl operation on the file.
	pub fn ioctl(
		&mut self,
		mem_space: IntSharedPtr<MemSpace>,
		request: u32,
		argp: *const c_void,
	) -> Result<u32, Errno> {
		let size = self.get_size();

		match &mut self.target {
			FDTarget::File(file) => {
				let guard = file.lock();
				let f = guard.get_mut();

				match f.get_file_content() {
					FileContent::Regular => match request {
						ioctl::FIONREAD => {
							let mem_space_guard = mem_space.lock();
							let count_ptr: SyscallPtr<c_int> = (argp as usize).into();
							let count_ref = count_ptr
								.get_mut(&mem_space_guard)?
								.ok_or_else(|| errno!(EFAULT))?;
							*count_ref = (size - min(size, self.curr_off)) as _;

							Ok(0)
						},

						_ => Err(errno!(EINVAL)),
					},

					_ => f.ioctl(mem_space, request, argp),
				}
			}

			FDTarget::Pipe(pipe) => {
				let guard = pipe.lock();
				guard.get_mut().ioctl(mem_space, request, argp)
			}

			FDTarget::Socket(sock) => {
				let guard = sock.lock();
				guard.get_mut().ioctl(mem_space, request, argp)
			}
		}
	}
}

impl IO for OpenFile {
	fn get_size(&self) -> u64 {
		if let FDTarget::File(f) = &self.target {
			f.get().lock().get().get_size()
		} else {
			0
		}
	}

	/// Note: on this specific implementation, the offset is ignored since `set_offset` has to be
	/// used to define it.
	fn read(&mut self, _: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		if !self.can_read() {
			return Err(errno!(EINVAL));
		}

		let (len, eof) = match &mut self.target {
			FDTarget::File(f) => {
				let guard = f.lock();
				let file = guard.get_mut();

				// Updating access timestamp
				let timestamp = time::get(TimestampScale::Second, true).unwrap_or(0);
				file.set_atime(timestamp); // TODO Only if the mountpoint has the option enabled
				file.sync()?; // TODO Lazy

				file.read(self.curr_off, buf)
			}

			FDTarget::Pipe(p) => {
				let guard = p.lock();
				guard.get_mut().read(self.curr_off, buf)
			}

			FDTarget::Socket(s) => {
				let guard = s.lock();
				guard.get_mut().read(self.curr_off, buf)
			}
		}?;

		self.curr_off += len as u64;
		Ok((len as _, eof))
	}

	/// Note: on this specific implementation, the offset is ignored since `set_offset` has to be
	/// used to define it.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		if !self.can_write() {
			return Err(errno!(EINVAL));
		}

		let len = match &mut self.target {
			FDTarget::File(f) => {
				let guard = f.lock();
				let file = guard.get_mut();

				// Appending if enabled
				if self.flags & O_APPEND != 0 {
					self.curr_off = file.get_size();
				}

				// Updating access timestamps
				let timestamp = time::get(TimestampScale::Second, true).unwrap_or(0);
				file.set_atime(timestamp); // TODO Only if the mountpoint has the option enabled
				file.set_mtime(timestamp);
				file.sync()?; // TODO Lazy

				file.write(self.curr_off, buf)
			}

			FDTarget::Pipe(p) => {
				let guard = p.lock();
				guard.get_mut().write(self.curr_off, buf)
			}

			FDTarget::Socket(s) => {
				let guard = s.lock();
				guard.get_mut().write(self.curr_off, buf)
			}
		}?;

		self.curr_off += len as u64;
		Ok(len as _)
	}

	fn poll(&mut self, mask: u32) -> Result<u32, Errno> {
		match &mut self.target {
			FDTarget::File(f) => {
				let guard = f.lock();
				let file = guard.get_mut();

				file.poll(mask)
			}

			FDTarget::Pipe(p) => {
				let guard = p.lock();
				guard.get_mut().poll(mask)
			}

			FDTarget::Socket(s) => {
				let guard = s.lock();
				guard.get_mut().poll(mask)
			}
		}
	}
}

impl Drop for OpenFile {
	fn drop(&mut self) {
		// Update references count
		match &self.target {
			FDTarget::File(file) => file.lock().get_mut().decrement_open(self.can_write()),
			FDTarget::Pipe(pipe) => pipe.lock().get_mut().decrement_open(self.can_write()),

			_ => {}
		}
	}
}
