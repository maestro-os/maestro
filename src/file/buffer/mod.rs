//! TODO doc

pub mod pipe;
pub mod socket;

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::process::mem_space::MemSpace;
use crate::syscall::ioctl;
use crate::util::FailableDefault;
use crate::util::container::hashmap::HashMap;
use crate::util::container::id_allocator::IDAllocator;
use crate::util::io::IO;
use crate::util::lock::Mutex;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// Trait representing a buffer.
pub trait Buffer: IO {
	/// Increments the number of open ends.
	///
	/// Arguments:
	/// - `read` tells whether the open end allows reading.
	/// - `write` tells whether the open end allows writing.
	fn increment_open(&mut self, read: bool, write: bool);

	/// Decrements the number of open ends.
	///
	/// Arguments:
	/// - `read` tells whether the open end allows reading.
	/// - `write` tells whether the open end allows writing.
	fn decrement_open(&mut self, read: bool, write: bool);

	/// Performs an ioctl operation on the file.
	///
	/// Arguments:
	/// - `mem_space` is the memory space on which pointers are to be dereferenced.
	/// - `request` is the ID of the request to perform.
	/// - `argp` is a pointer to the argument.
	fn ioctl(
		&mut self,
		mem_space: IntSharedPtr<MemSpace>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> Result<u32, Errno>;
}

/// All the system's buffer. The key is the location of the file associated with the
/// entry.
static BUFFERS: Mutex<HashMap<FileLocation, SharedPtr<dyn Buffer>>>
	= Mutex::new(HashMap::new());
/// Buffer ID allocator.
static ID_ALLOCATOR: Mutex<Option<IDAllocator>> = Mutex::new(None);

/// TODO doc
fn id_allocator_do<T, F>(f: F) -> Result<T, Errno>
	where F: FnOnce(&mut IDAllocator) -> Result<T, Errno> {
	let id_allocator_guard = ID_ALLOCATOR.lock();
	let id_allocator = id_allocator_guard.get_mut();

	let id_allocator = match id_allocator {
		Some(id_allocator) => id_allocator,
		None => {
			*id_allocator = Some(IDAllocator::new(65536)?);
			id_allocator.as_mut().unwrap()
		},
	};

	f(id_allocator)
}

/// Returns the buffer associated with the file at location `loc`.
///
/// If the buffer doesn't exist, the function creates it.
pub fn get(loc: &FileLocation) -> Option<SharedPtr<dyn Buffer>> {
	let buffers_guard = BUFFERS.lock();
	let buffers = buffers_guard.get_mut();

	buffers.get(loc).cloned()
}

/// Returns the buffer associated with the file at location `loc`.
///
/// If the buffer doesn't exist, the function registers a new default buffer.
pub fn get_or_default<B: Buffer + FailableDefault + 'static>(
	loc: &FileLocation
) -> Result<SharedPtr<dyn Buffer>, Errno> {
	let buffers_guard = BUFFERS.lock();
	let buffers = buffers_guard.get_mut();

	match buffers.get(loc).cloned() {
		Some(buff) => Ok(buff),

		None => {
			let buff = SharedPtr::new(B::failable_default()?)?;
			buffers.insert(loc.clone(), buff.clone())?;

			Ok(buff)
		},
	}
}

/// Registers a new buffer.
///
/// If no location is provided, the function allocates a virtual location.
/// If every possible virtual locations are used, the function returns an error.
///
/// Arguments:
/// - `loc` is the location of the file.
/// - `buff` is the buffer to be registered.
///
/// The function returns the location associated with the buffer.
pub fn register(
	loc: Option<FileLocation>,
	buff: SharedPtr<dyn Buffer>
) -> Result<FileLocation, Errno> {
	let loc = id_allocator_do(|id_allocator| match loc {
		Some(loc) => {
			if let FileLocation::Virtual { id } = loc {
				id_allocator.set_used(id);
			}

			Ok(loc)
		}

		None => Ok(FileLocation::Virtual {
			id: id_allocator.alloc(None)?,
		})
	})?;

	let buffers_guard = BUFFERS.lock();
	let buffers = buffers_guard.get_mut();
	buffers.insert(loc.clone(), buff)?;

	Ok(loc)
}

/// Frees the buffer with the given location `loc`.
///
/// If the location doesn't exist or doesn't match any existing buffer, the function does nothing.
pub fn release(loc: &FileLocation) {
	let buffers_guard = BUFFERS.lock();
	let buffers = buffers_guard.get_mut();

	let _ = buffers.remove(loc);

	if let FileLocation::Virtual { id } = loc {
		let _ = id_allocator_do(|id_allocator| {
			id_allocator.free(*id);
			Ok(())
		});
	}
}
