//! An open file description is a structure pointing to a file, allowing to
//! perform operations on it. It is pointed to by file descriptors.

use crate::device;
use crate::device::DeviceType;
use crate::errno;
use crate::errno::Errno;
use crate::file::buffer;
use crate::file::mountpoint;
use crate::file::vfs;
use crate::file::DeviceID;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::MemSpace;
use crate::process::Process;
use crate::syscall::ioctl;
use crate::time::clock;
use crate::time::clock::CLOCK_MONOTONIC;
use crate::time::unit::TimestampScale;
use crate::util::container::hashmap::HashMap;
use crate::util::io::IO;
use crate::util::lock::IntMutex;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use core::cmp::min;
use core::ffi::c_int;
use core::ffi::c_void;

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
static OPEN_FILES: Mutex<HashMap<FileLocation, Arc<Mutex<OpenFile>>>> = Mutex::new(HashMap::new());

// TODO Keep a different references counter for read and write.
// And increment/decrement on open and close (using FD's flags)

/// An open file description.
///
/// This structure is pointed to by file descriptors and point to files.
/// They exist to ensure several file descriptors can share the same open file.
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
	/// If the location doesn't exist or if the file isn't open, the function returns `None`.
	pub fn get(location: &FileLocation) -> Option<Arc<Mutex<Self>>> {
		OPEN_FILES.lock().get(location).cloned()
	}

	/// Creates a new open file description and inserts it into the open files list.
	///
	/// Arguments:
	/// - `location` is the location of the file to be openned.
	/// - `flags` is the open file's set of flags.
	///
	/// If an open file already exists for this location, the function add the given flags to the
	/// already existing instance and returns it.
	pub fn new(location: FileLocation, flags: i32) -> Result<Arc<Mutex<Self>>, Errno> {
		let open_file_mutex = match Self::get(&location) {
			Some(open_file_mutex) => {
				{
					let mut open_file = open_file_mutex.lock();

					let read =
						open_file.can_read() || flags & 0b11 == O_RDONLY || flags & 0b11 == O_RDWR;
					let write = open_file.can_write()
						|| flags & 0b11 == O_WRONLY
						|| flags & 0b11 == O_RDWR;

					let mut new_flags = (open_file.flags & !0b11) | (flags & !0b11);
					if read && write {
						new_flags |= O_RDWR;
					} else if read {
						new_flags |= O_RDONLY;
					} else if write {
						new_flags |= O_WRONLY;
					}

					open_file.flags = new_flags;
				}

				open_file_mutex
			}

			None => {
				let open_file = Arc::new(Mutex::new(Self {
					location: location.clone(),

					flags,
					curr_off: 0,

					ref_count: 0,
				}))?;

				OPEN_FILES.lock().insert(location, open_file.clone())?;

				open_file
			}
		};

		Ok(open_file_mutex)
	}

	/// Increments the number of references to the file with the given location.
	///
	/// Arguments:
	/// - `location` is the location of the file.
	/// - `read` tells whether the file descriptor is open for reading.
	/// - `write` tells whether the file descriptor is open for writing.
	pub fn open(
		location: FileLocation,
		read: bool,
		write: bool,
	) -> Result<Arc<Mutex<Self>>, Errno> {
		let open_file_mutex = Self::get(&location).ok_or_else(|| errno!(ENOENT))?;

		{
			let mut open_file = open_file_mutex.lock();
			open_file.ref_count += 1;

			// If the file points to a buffer, increment the number of open ends
			if let Some(buff_mutex) = buffer::get(&open_file.location) {
				let mut buff = buff_mutex.lock();
				buff.increment_open(read, write);
			}
		}

		Ok(open_file_mutex)
	}

	/// Decrements the reference counter of the open file for the given location.
	///
	/// If the references count reaches zero, the function removes the open file.
	/// If the file has a virtual location, this location is also freed.
	///
	/// Arguments:
	/// - `location` is the location of the file.
	/// - `read` tells whether the file descriptor is open for reading.
	/// - `write` tells whether the file descriptor is open for writing.
	///
	/// If the file is not open, the function does nothing.
	pub fn close(location: &FileLocation, read: bool, write: bool) {
		let mut open_files = OPEN_FILES.lock();

		let Some(open_file_mutex) = open_files.get(location) else {
			return;
		};
		let mut open_file = open_file_mutex.lock();

		// If the file points to a buffer, decrement the number of open ends
		if let Some(buff_mutex) = buffer::get(&open_file.location) {
			let mut buff = buff_mutex.lock();
			buff.decrement_open(read, write);
		}

		open_file.ref_count -= 1;
		if open_file.ref_count == 0 {
			drop(open_file);

			open_files.remove(location);
			buffer::release(location);
		}
	}

	/// Returns the location of the open file.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Returns the file.
	///
	/// The name of the file is not set since it cannot be known from this structure.
	pub fn get_file(&self) -> Result<Arc<Mutex<File>>, Errno> {
		let vfs_mutex = vfs::get();
		let mut vfs = vfs_mutex.lock();
		let vfs = vfs.as_mut().unwrap();

		vfs.get_file_by_location(&self.location)
	}

	/// Returns the file flags.
	pub fn get_flags(&self) -> i32 {
		self.flags
	}

	/// Sets the open file flags.
	///
	/// File access mode (`O_RDONLY`, `O_WRONLY`, `O_RDWR`) and file creation flags
	/// (`O_CREAT`, `O_EXCL`, `O_NOCTTY`, `O_TRUNC`) are ignored.
	pub fn set_flags(&mut self, flags: i32) {
		let ignored_flags = 0b11 | O_RDWR | O_CREAT | O_EXCL | O_NOCTTY | O_TRUNC;
		self.flags = (self.flags & ignored_flags) | (flags & !ignored_flags);
	}

	/// Tells whether the open file can be read from.
	pub fn can_read(&self) -> bool {
		!matches!(self.flags & 0b11, O_WRONLY)
	}

	/// Tells whether the open file can be written to.
	pub fn can_write(&self) -> bool {
		matches!(self.flags & 0b11, O_WRONLY | O_RDWR)
	}

	/// Tells whether the atime is updated on access.
	fn is_atime_updated(&self) -> bool {
		let Some(mp) = self.location.get_mountpoint() else {
			return true;
		};
		let mp_guard = mp.lock();

		mp_guard.get_flags() & mountpoint::FLAG_NOATIME != 0
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
		mem_space: Arc<IntMutex<MemSpace>>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> Result<u32, Errno> {
		let file_mutex = self.get_file()?;
		let mut file = file_mutex.lock();

		match file.get_content() {
			FileContent::Regular => match request.get_old_format() {
				ioctl::FIONREAD => {
					let mut mem_space_guard = mem_space.lock();
					let count_ptr: SyscallPtr<c_int> = (argp as usize).into();
					let count_ref = count_ptr
						.get_mut(&mut mem_space_guard)?
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

	/// Adds the given process to the list of processes waiting on the file.
	///
	/// The function sets the state of the process to `Sleeping`.
	/// When the event occurs, the process will be woken up.
	///
	/// `mask` is the mask of poll event to wait for.
	///
	/// If the file cannot block, the function does nothing.
	pub fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> Result<(), Errno> {
		let file_mutex = self.get_file()?;
		let file = file_mutex.lock();

		match file.get_content() {
			FileContent::Fifo | FileContent::Socket => {
				if let Some(buff_mutex) = buffer::get(self.get_location()) {
					let mut buff = buff_mutex.lock();
					return buff.add_waiting_process(proc, mask);
				}
			}

			FileContent::BlockDevice {
				major,
				minor,
			} => {
				let dev_mutex = device::get(&DeviceID {
					type_: DeviceType::Block,
					major: *major,
					minor: *minor,
				});

				if let Some(dev_mutex) = dev_mutex {
					let mut dev = dev_mutex.lock();
					return dev.get_handle().add_waiting_process(proc, mask);
				}
			}

			FileContent::CharDevice {
				major,
				minor,
			} => {
				let dev_mutex = device::get(&DeviceID {
					type_: DeviceType::Char,
					major: *major,
					minor: *minor,
				});

				if let Some(dev_mutex) = dev_mutex {
					let mut dev = dev_mutex.lock();
					return dev.get_handle().add_waiting_process(proc, mask);
				}
			}

			_ => {}
		}

		Ok(())
	}
}

impl IO for OpenFile {
	fn get_size(&self) -> u64 {
		self.get_file().map(|f| f.lock().get_size()).unwrap_or(0)
	}

	/// Note: on this specific implementation, the offset is ignored since
	/// `set_offset` has to be used to define it.
	fn read(&mut self, _off: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		if !self.can_read() {
			return Err(errno!(EINVAL));
		}

		let file_mutex = self.get_file()?;
		let mut file = file_mutex.lock();

		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Updating access timestamp
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);
		if self.is_atime_updated() {
			file.atime = timestamp;
			file.sync()?; // TODO Lazy
		}

		let (len, eof) = file.read(self.curr_off, buf)?;

		self.curr_off += len;
		Ok((len as _, eof))
	}

	/// Note: on this specific implementation, the offset is ignored since
	/// `set_offset` has to be used to define it.
	fn write(&mut self, _off: u64, buf: &[u8]) -> Result<u64, Errno> {
		if !self.can_write() {
			return Err(errno!(EINVAL));
		}

		let file_mutex = self.get_file()?;
		let mut file = file_mutex.lock();

		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Appending if enabled
		if self.flags & O_APPEND != 0 {
			self.curr_off = file.get_size();
		}

		// Updating access timestamps
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);
		if self.is_atime_updated() {
			file.atime = timestamp;
		}
		file.mtime = timestamp;
		file.sync()?; // TODO Lazy

		let len = file.write(self.curr_off, buf)?;

		self.curr_off += len;
		Ok(len as _)
	}

	fn poll(&mut self, mask: u32) -> Result<u32, Errno> {
		let file_mutex = self.get_file()?;
		let mut file = file_mutex.lock();

		file.poll(mask)
	}
}
