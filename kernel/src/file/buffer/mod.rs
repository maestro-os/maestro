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

//! A buffer is an FIFO resource which may be blocking. The resource is represented by a file.

pub mod pipe;
pub mod socket;

use crate::file::{blocking::WaitQueue, fs::NodeOps, FileLocation};
use core::{alloc::AllocError, any::Any};
use utils::{
	collections::{hashmap::HashMap, id_allocator::IDAllocator},
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::arc::Arc,
	TryDefault,
};

/// Trait representing a buffer.
pub trait Buffer: Any + NodeOps {
	/// Returns the capacity in bytes of the buffer.
	fn get_capacity(&self) -> usize;

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
}

/// All the system's buffer. The key is the location of the file associated with the
/// entry.
static BUFFERS: Mutex<HashMap<FileLocation, Arc<Mutex<dyn Buffer>>>> = Mutex::new(HashMap::new());
/// Buffer ID allocator.
static ID_ALLOCATOR: Mutex<Option<IDAllocator>> = Mutex::new(None);

/// Executes the given closure, passing the ID allocator.
///
/// This function allows to access the ID allocator without bothering about its mutex.
///
/// If the ID allocator is not initialized, the function initializes it.
fn id_allocator_do<T, F>(f: F) -> EResult<T>
where
	F: FnOnce(&mut IDAllocator) -> EResult<T>,
{
	let mut id_allocator = ID_ALLOCATOR.lock();
	let id_allocator = match &mut *id_allocator {
		Some(a) => a,
		None => id_allocator.insert(IDAllocator::new(65536)?),
	};
	f(id_allocator)
}

/// Returns the buffer associated with the file at location `loc`.
///
/// If the buffer doesn't exist, the function creates it.
pub fn get(loc: &FileLocation) -> Option<Arc<Mutex<dyn Buffer>>> {
	let buffers = BUFFERS.lock();
	buffers.get(loc).cloned()
}

/// Returns the buffer associated with the file at location `loc`.
///
/// If the buffer doesn't exist, the function registers a new default buffer.
pub fn get_or_default<B: Buffer + TryDefault<Error = AllocError> + 'static>(
	loc: &FileLocation,
) -> AllocResult<Arc<Mutex<dyn Buffer>>> {
	let mut buffers = BUFFERS.lock();
	match buffers.get(loc).cloned() {
		Some(buff) => Ok(buff),
		None => {
			let buff = Arc::new(Mutex::new(B::try_default()?))?;
			buffers.insert(*loc, buff.clone())?;
			Ok(buff)
		}
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
pub fn register(loc: Option<FileLocation>, buff: Arc<Mutex<dyn Buffer>>) -> EResult<FileLocation> {
	let loc = id_allocator_do(|id_allocator| match loc {
		Some(loc) => {
			if let FileLocation::Virtual(id) = loc {
				id_allocator.set_used(id);
			}
			Ok(loc)
		}
		None => Ok(FileLocation::Virtual(id_allocator.alloc(None)?)),
	})?;
	let mut buffers = BUFFERS.lock();
	buffers.insert(loc, buff)?;
	Ok(loc)
}

/// Frees the buffer with the given location `loc`.
///
/// If the location doesn't exist or doesn't match any existing buffer, the function does nothing.
pub fn release(loc: &FileLocation) {
	let mut buffers = BUFFERS.lock();
	buffers.remove(loc);
	if let FileLocation::Virtual(id) = loc {
		let _ = id_allocator_do(|id_allocator| {
			id_allocator.free(*id);
			Ok(())
		});
	}
}
