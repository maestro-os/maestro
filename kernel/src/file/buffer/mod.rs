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

use crate::{
	file::{fs::NodeOps, wait_queue::WaitQueue, FileLocation, Stat},
	syscall::ioctl::Request,
};
use core::{alloc::AllocError, any::Any, ffi::c_void};
use utils::{
	collections::hashmap::HashMap,
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::arc::Arc,
	TryDefault,
};

/// Trait representing a buffer.
pub trait BufferOps: Any + NodeOps {
	/// Increments the number of open ends.
	///
	/// Arguments:
	/// - `read` tells whether the open end allows reading.
	/// - `write` tells whether the open end allows writing.
	fn acquire(&self, read: bool, write: bool);

	/// Decrements the number of open ends.
	///
	/// Arguments:
	/// - `read` tells whether the open end allows reading.
	/// - `write` tells whether the open end allows writing.
	fn release(&self, read: bool, write: bool);
}

/// A buffer.
#[derive(Clone, Debug)]
pub struct Buffer(pub Arc<dyn BufferOps>);

impl Buffer {
	/// Creates a new instance with the given buffer type.
	pub fn new<B: BufferOps + TryDefault<Error = AllocError> + 'static>(
		buf: B,
	) -> AllocResult<Self> {
		Ok(Self(Arc::new(buf)?))
	}
}

impl NodeOps for Buffer {
	fn get_stat(&self, loc: &FileLocation) -> EResult<Stat> {
		self.0.get_stat(loc)
	}

	fn poll(&self, loc: &FileLocation, mask: u32) -> EResult<u32> {
		self.0.poll(loc, mask)
	}

	fn ioctl(&self, loc: &FileLocation, request: Request, argp: *const c_void) -> EResult<u32> {
		self.0.ioctl(loc, request, argp)
	}

	fn read_content(&self, loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		self.0.read_content(loc, off, buf)
	}

	fn write_content(&self, loc: &FileLocation, off: u64, buf: &[u8]) -> EResult<usize> {
		self.0.write_content(loc, off, buf)
	}
}

/// Buffers associated with filesystem locations (example: fifo and socket files).
///
/// The key is the location of the file associated with the entry.
pub static BUFFERS: Mutex<HashMap<FileLocation, Buffer>> = Mutex::new(HashMap::new());
