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

//! An open file description is a structure pointing to a file, allowing to
//! perform operations on it. It is pointed to by file descriptors.

use crate::{
	device,
	device::DeviceType,
	file::{buffer, mountpoint, DeviceID, File, FileContent, FileLocation},
	process::{
		mem_space::{ptr::SyscallPtr, MemSpace},
		Process,
	},
	syscall::ioctl,
	time::{clock, clock::CLOCK_MONOTONIC, unit::TimestampScale},
};
use core::{
	cmp::min,
	ffi::{c_int, c_void},
};
use utils::{
	collections::hashmap::HashMap,
	errno,
	errno::EResult,
	io::IO,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

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

// TODO move buffer handling to `FileContent`?

/// Counts the number of time each file is open.
static OPEN_FILES: Mutex<HashMap<FileLocation, usize>> = Mutex::new(HashMap::new());

/// An open file description.
///
/// This structure is pointed to by file descriptors and point to files.
/// They exist to ensure several file descriptors can share the same open file.
#[derive(Debug)]
pub struct OpenFile {
	/// The open file. This is an option to allow easier dropping implementation.
	file: Option<Arc<Mutex<File>>>,
	/// The file's location. This field is necessary to avoid locking the file's mutex each time
	/// the location is required.
	location: FileLocation,
	/// The open file description's flags.
	flags: i32,

	/// The current offset in the file.
	/// If pointing to a directory, this is the offset in directory entries.
	curr_off: u64,
}

impl OpenFile {
	/// Creates a new open file description and inserts it into the open files list.
	///
	/// Arguments:
	/// - `file` is the open file
	/// - `flags` is the open file's set of flags
	///
	/// If an open file already exists for this location, the function add the given flags to the
	/// already existing instance and returns it.
	pub fn new(file: Arc<Mutex<File>>, flags: i32) -> EResult<Self> {
		let location = file.lock().get_location().clone();
		let s = Self {
			file: Some(file),
			location: location.clone(),
			flags,

			curr_off: 0,
		};

		// Update the open file counter
		{
			let mut open_files = OPEN_FILES.lock();
			if let Some(count) = open_files.get_mut(&location) {
				*count += 1;
			} else {
				open_files.insert(location.clone(), 1)?;
			}
		}

		// If the file points to a buffer, increment the number of open ends
		if let Some(buff_mutex) = buffer::get(&location) {
			let mut buff = buff_mutex.lock();
			buff.increment_open(s.can_read(), s.can_write());
		}

		Ok(s)
	}

	/// Tells whether the file at the given location is open.
	pub fn is_open(loc: &FileLocation) -> bool {
		OPEN_FILES.lock().contains_key(loc)
	}

	/// Decrements the reference counter of the open file for the given location.
	///
	/// If the references count reaches zero, the function removes the open file.
	/// If the file has a virtual location, this location is also freed.
	///
	/// Arguments:
	/// - `location` is the location of the file.
	/// - `read` tells whether the file descriptor is open for reading
	/// - `write` tells whether the file descriptor is open for writing
	///
	/// If the file is not open, the function does nothing.
	pub fn close(mut self) -> EResult<()> {
		// Close file if this is the last reference to it
		let Some(file) = self.file.take().and_then(Arc::into_inner) else {
			return Ok(());
		};
		file.into_inner().close()
	}

	/// Returns the file.
	///
	/// The name of the file is not set since it cannot be known from this structure.
	pub fn get_file(&self) -> &Arc<Mutex<File>> {
		self.file.as_ref().unwrap()
	}

	/// Returns the location of the file.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
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

	/// Tells whether the access time (`atime`) must be updated on access.
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
	) -> EResult<u32> {
		let mut file = self.get_file().lock();
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

				_ => Err(errno!(ENOTTY)),
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
	pub fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> EResult<()> {
		let file = self.get_file().lock();
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
		self.get_file().lock().get_size()
	}

	/// Note: on this specific implementation, the offset is ignored since
	/// `set_offset` has to be used to define it.
	fn read(&mut self, _off: u64, buf: &mut [u8]) -> EResult<(u64, bool)> {
		if !self.can_read() {
			return Err(errno!(EINVAL));
		}

		let mut file = self.file.as_ref().unwrap().lock();
		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Update access timestamp
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
	fn write(&mut self, _off: u64, buf: &[u8]) -> EResult<u64> {
		if !self.can_write() {
			return Err(errno!(EINVAL));
		}

		let mut file = self.file.as_ref().unwrap().lock();
		if matches!(file.get_content(), FileContent::Directory(_)) {
			return Err(errno!(EISDIR));
		}

		// Append if enabled
		if self.flags & O_APPEND != 0 {
			self.curr_off = file.get_size();
		}

		// Update access timestamps
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

	fn poll(&mut self, mask: u32) -> EResult<u32> {
		self.get_file().lock().poll(mask)
	}
}

impl Drop for OpenFile {
	fn drop(&mut self) {
		// If the file points to a buffer, decrement the number of open ends
		if let Some(buff_mutex) = buffer::get(&self.location) {
			let mut buff = buff_mutex.lock();
			buff.decrement_open(self.can_read(), self.can_write());
		}
		// Update the open file counter
		{
			let mut open_files = OPEN_FILES.lock();
			if let Some(count) = open_files.get_mut(&self.location) {
				*count -= 1;
				if *count == 0 {
					open_files.remove(&self.location);
				}
			}
		}
	}
}
