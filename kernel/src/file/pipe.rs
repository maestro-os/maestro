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

//! A pipe is an object that links two file descriptors together. One reading
//! and another writing, with a buffer in between.

use crate::{
	file::{File, FileType, O_NONBLOCK, Stat, fs::FileOps, wait_queue::WaitQueue},
	memory::{
		ring_buffer::RingBuffer,
		user::{UserPtr, UserSlice},
	},
	process::{Process, signal::Signal},
	sync::mutex::Mutex,
	syscall::{FromSyscallArg, ioctl},
};
use core::{
	ffi::{c_int, c_void},
	intrinsics::unlikely,
	num::NonZeroUsize,
};
use utils::{
	errno,
	errno::{AllocResult, EResult},
	limits::PIPE_BUF,
};

#[derive(Debug)]
struct PipeInner {
	/// The pipe's buffer.
	buffer: RingBuffer,
	/// The number of readers on the pipe.
	readers: usize,
	/// The number of writers on the pipe.
	writers: usize,
}

/// Representing a FIFO buffer.
#[derive(Debug)]
pub struct PipeBuffer {
	/// Inner with locking.
	inner: Mutex<PipeInner>,
	/// The queue of processing waiting to read from the pipe.
	rd_queue: WaitQueue,
	/// The queue of processing waiting to write to the pipe.
	wr_queue: WaitQueue,
}

impl PipeBuffer {
	/// Creates a new instance.
	pub fn new() -> AllocResult<Self> {
		Ok(Self {
			inner: Mutex::new(PipeInner {
				buffer: RingBuffer::new(NonZeroUsize::new(PIPE_BUF).unwrap())?,
				readers: 0,
				writers: 0,
			}),
			rd_queue: WaitQueue::default(),
			wr_queue: WaitQueue::default(),
		})
	}

	/// Returns the capacity of the pipe in bytes.
	pub fn get_capacity(&self) -> usize {
		PIPE_BUF
	}
}

impl FileOps for PipeBuffer {
	fn get_stat(&self, _file: &File) -> EResult<Stat> {
		Ok(Stat {
			mode: FileType::Fifo.to_mode() | 0o666,
			..Default::default()
		})
	}

	fn acquire(&self, file: &File) {
		let mut inner = self.inner.lock();
		if file.can_read() {
			inner.readers += 1;
		}
		if file.can_write() {
			inner.writers += 1;
		}
	}

	fn release(&self, file: &File) {
		let mut inner = self.inner.lock();
		if file.can_read() {
			inner.readers -= 1;
		}
		if file.can_write() {
			inner.writers -= 1;
		}
		if (inner.readers == 0) != (inner.writers == 0) {
			self.rd_queue.wake_all();
			self.wr_queue.wake_all();
		}
	}

	fn poll(&self, _file: &File, _mask: u32) -> EResult<u32> {
		todo!()
	}

	fn ioctl(&self, _file: &File, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::FIONREAD => {
				let len = self.inner.lock().buffer.get_data_len() as c_int;
				let count_ptr = UserPtr::from_ptr(argp as usize);
				count_ptr.copy_to_user(&len)?;
			}
			_ => return Err(errno!(ENOTTY)),
		}
		Ok(0)
	}

	fn read(&self, file: &File, _off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		if unlikely(buf.is_empty()) {
			return Ok(0);
		}
		let len = self.rd_queue.wait_until(|| {
			let mut inner = self.inner.lock();
			let len = match inner.buffer.read(buf) {
				Ok(l) => l,
				Err(e) => return Some(Err(e)),
			};
			if len > 0 {
				self.wr_queue.wake_next();
				return Some(Ok(len));
			}
			// Nothing to read
			if inner.writers == 0 {
				return Some(Ok(0));
			}
			if file.get_flags() & O_NONBLOCK != 0 {
				Some(Err(errno!(EAGAIN)))
			} else {
				None
			}
		})??;
		Ok(len)
	}

	fn write(&self, file: &File, _off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		if unlikely(buf.is_empty()) {
			return Ok(0);
		}
		let len = self.wr_queue.wait_until(|| {
			let mut inner = self.inner.lock();
			if inner.readers == 0 {
				Process::current().kill(Signal::SIGPIPE);
				return Some(Err(errno!(EPIPE)));
			}
			let len = match inner.buffer.write(buf) {
				Ok(l) => l,
				Err(e) => return Some(Err(e)),
			};
			if len > 0 {
				self.rd_queue.wake_next();
				return Some(Ok(len));
			}
			// No space left to write
			if file.get_flags() & O_NONBLOCK != 0 {
				Some(Err(errno!(EAGAIN)))
			} else {
				None
			}
		})??;
		Ok(len)
	}
}
