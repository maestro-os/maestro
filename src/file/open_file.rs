//! An open file description is a structure pointing to a file, allowing to
//! perform operations on it. It is pointed to by file descriptors.

use core::cmp::min;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::vfs;
use crate::file::virt;
use crate::process::mem_space::MemSpace;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::syscall::ioctl;
use crate::time::unit::TimestampScale;
use crate::time;
use crate::types::c_int;
use crate::util::container::hashmap::HashMap;
use crate::util::io::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// Read only.
pub const O_RDONLY: i32 = 0b00000000000000000000000000000000;
/// Write only.
pub const O_WRONLY: i32 = 0b00000000000000000000000000000001;
/// Read and write.
pub const O_RDWR: i32 = 0b00000000000000000000000000000010;
/// At each write operations, the cursor is placed at the end of the file so the
/// data is appended.
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
/// When using `write`, the data has been transfered to the hardware before
/// returning.
pub const O_SYNC: i32 = 0b00000000000100000001000000000000;
/// If the file already exists, truncate it to length zero.
pub const O_TRUNC: i32 = 0b00000000000000000000001000000000;

/// The list of currently open files.
static OPEN_FILES: Mutex<HashMap<FileLocation, SharedPtr<OpenFile>>> = Mutex::new(HashMap::new());

/// An open file description. This structure is pointed to by file descriptors
/// and point to files. They exist to ensure several file descriptors can share
/// the same open file.
#[derive(Debug)]
pub struct OpenFile {
	/// The location of the file.
	location: FileLocation,

	/// The open file description's flags.
	flags: i32,
	/// The current offset in the file.
	/// If pointing to a directory, this is the offset in directory entries.
	curr_off: u64,

	/// The number of concurrent file descriptors pointing the the current file.
	ref_count: usize,
}

impl OpenFile {
	/// Returns the open file at the given location.
	///
	/// If the location doesn't exist or if the file isn't open, the function returns None.
	pub fn get(location: &FileLocation) -> Option<SharedPtr<Self>> {
		OPEN_FILES.lock()
			.get()
			.get(location)
			.cloned()
	}

	/// Creates a new open file description and inserts it into the open files list.
	///
	/// Arguments:
	/// - `location` is the location of the file to be openned.
	/// - `flags` is the open file's set of flags.
	pub fn open(
		location: FileLocation,
		flags: i32,
	) -> Result<SharedPtr<Self>, Errno> {
		let open_file_mutex = match Self::get(&location) {
			Some(open_file) => open_file,

			// If not open, create a new instance
			None => {
				let open_file = SharedPtr::new(Self {
					location: location.clone(),

					flags,
					curr_off: 0,

					ref_count: 0,
				})?;
				OPEN_FILES.lock()
					.get_mut()
					.insert(location, open_file.clone())?;

				open_file
			},
		};

		{
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get_mut();
			open_file.ref_count += 1;

			// If the file points to a virtual resource, increment the number of open ends
			if let Some(res_mutex) = virt::get_resource(&open_file.location) {
				let res_guard = res_mutex.lock();
				let res = res_guard.get_mut();

				res.increment_open(open_file.can_write());
			}
		}

		Ok(open_file_mutex)
	}

	/// Returns the location of the open file.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Returns the file.
	///
	/// The name of the file is not set since it cannot be known from this structure.
	pub fn get_file(&self) -> Result<SharedPtr<File>, Errno> {
		let vfs_mutex = vfs::get();
		let vfs_guard = vfs_mutex.lock();
		let vfs = vfs_guard.get_mut().as_mut().unwrap();

		vfs.get_file_by_location(&self.location)
	}

	/// Returns the file flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Sets the open file flags.
	///
	/// File access mode (O_RDONLY, O_WRONLY, O_RDWR) and file creation flags
	/// (O_CREAT, O_EXCL, O_NOCTTY, O_TRUNC) are ignored.
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
		let file_mutex = self.get_file()?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		match file.get_content() {
			FileContent::Regular => match request {
				ioctl::FIONREAD => {
					let mem_space_guard = mem_space.lock();
					let count_ptr: SyscallPtr<c_int> = (argp as usize).into();
					let count_ref = count_ptr
						.get_mut(&mem_space_guard)?
						.ok_or_else(|| errno!(EFAULT))?;

					let size = file.get_size();
					*count_ref = (size - min(size, self.curr_off)) as _;

					Ok(0)
				}

				_ => Err(errno!(EINVAL)),
			},

			_ => file.ioctl(mem_space, request, argp),
		}
	}

	/// Decrements the reference counter of the open file for the given location.
	///
	/// If the file is not open, the function does nothing.
	pub fn close(location: &FileLocation) {
		let open_files_guard = OPEN_FILES.lock();
		let open_files = open_files_guard.get_mut();

		let Some(open_file_mutex) = open_files.get(location) else {
			return;
		};
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get_mut();

		open_file.ref_count -= 1;
		if open_file.ref_count <= 0 {
			drop(open_file_guard);

			open_files.remove(location);
		}
	}
}

impl IO for OpenFile {
	fn get_size(&self) -> u64 {
		self.get_file()
			.map(|f| f.lock().get().get_size())
			.unwrap_or(0)
	}

	/// Note: on this specific implementation, the offset is ignored since
	/// `set_offset` has to be used to define it.
	fn read(&mut self, _: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		if !self.can_read() {
			return Err(errno!(EINVAL));
		}

		let file_mutex = self.get_file()?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Updating access timestamp
		let timestamp = time::get(TimestampScale::Second, true).unwrap_or(0);
		file.set_atime(timestamp); // TODO Only if the mountpoint has the option enabled
		file.sync()?; // TODO Lazy

		let (len, eof) = file.read(self.curr_off, buf)?;

		self.curr_off += len as u64;
		Ok((len as _, eof))
	}

	/// Note: on this specific implementation, the offset is ignored since
	/// `set_offset` has to be used to define it.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		if !self.can_write() {
			return Err(errno!(EINVAL));
		}

		let file_mutex = self.get_file()?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Appending if enabled
		if self.flags & O_APPEND != 0 {
			self.curr_off = file.get_size();
		}

		// Updating access timestamps
		let timestamp = time::get(TimestampScale::Second, true).unwrap_or(0);
		file.set_atime(timestamp); // TODO Only if the mountpoint has the option enabled
		file.set_mtime(timestamp);
		file.sync()?; // TODO Lazy

		let len = file.write(self.curr_off, buf)?;

		self.curr_off += len as u64;
		Ok(len as _)
	}

	fn poll(&mut self, mask: u32) -> Result<u32, Errno> {
		let file_mutex = self.get_file()?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		file.poll(mask)
	}
}

impl Drop for OpenFile {
	fn drop(&mut self) {
		// If the file points to a virtual resource, decrement the number of open ends
		if let Some(res_mutex) = virt::get_resource(&self.location) {
			let res_guard = res_mutex.lock();
			let res = res_guard.get_mut();

			res.decrement_open(self.can_write());
		}
	}
}
