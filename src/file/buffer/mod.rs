//! TODO doc

pub mod pipe;
pub mod socket;

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::process::mem_space::MemSpace;
use crate::util::container::hashmap::HashMap;
use crate::util::io::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// Trait representing a buffer.
pub trait Buffer: IO {
	/// Increments the number of open ends.
	///
	/// `write` tells whether writing is enabled on the opened end.
	fn increment_open(&mut self, write: bool);

	/// Decrements the number of open ends.
	///
	/// `write` tells whether writing is enabled on the closed end.
	fn decrement_open(&mut self, write: bool);

	/// Performs an ioctl operation on the file.
	///
	/// Arguments:
	/// - `mem_space` is the memory space on which pointers are to be dereferenced.
	/// - `request` is the ID of the request to perform.
	/// - `argp` is a pointer to the argument.
	fn ioctl(
		&mut self,
		mem_space: IntSharedPtr<MemSpace>,
		request: u32,
		argp: *const c_void,
	) -> Result<u32, Errno>;
}

/// All the system's buffer. The key is the location of the file associated with the
/// entry.
static RESOURCES: Mutex<HashMap<FileLocation, SharedPtr<dyn Buffer>>>
	= Mutex::new(HashMap::new());

/// Returns the pipe associated with the file at location `loc`.
///
/// If the pipe doesn't exist, the function creates it.
pub fn get(loc: &FileLocation) -> Option<SharedPtr<dyn Buffer>> {
	let buffers_guard = RESOURCES.lock();
	let buffers = buffers_guard.get_mut();

	buffers.get(loc).cloned()
}

/// Registers a new buffer.
///
/// If no location is provided, the function allocates a virtual location.
/// If every possible virtual locations are used (unlikely), the function returns an error.
///
/// `res` is the buffer to be registered.
///
/// The function returns the location associated with the buffer.
pub fn register(
	loc: Option<FileLocation>,
	res: SharedPtr<dyn Buffer>
) -> Result<FileLocation, Errno> {
	// TODO alloc location
	// TODO register buffer with location
	todo!();
}

/// Frees the buffer with the given location `loc`.
///
/// If the location doesn't exist, the function does nothing.
pub fn release(loc: &FileLocation) {
	let buffers_guard = RESOURCES.lock();
	let buffers = buffers_guard.get_mut();

	let _ = buffers.remove(loc);

	// TODO free location
	todo!();
}
