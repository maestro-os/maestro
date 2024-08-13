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

use super::Buffer;
use crate::{
	file::{
		buffer::WaitQueue,
		fs::{Filesystem, NodeOps},
		FileType, INode, Stat,
	},
	limits,
	process::{mem_space::copy::SyscallPtr, signal::Signal, Process},
	syscall::{ioctl, FromSyscallArg},
};
use core::{
	ffi::{c_int, c_void},
	intrinsics::unlikely,
};
use utils::{
	collections::{ring_buffer::RingBuffer, vec::Vec},
	errno,
	errno::EResult,
	vec, TryDefault,
};

/// Representing a FIFO buffer.
#[derive(Debug)]
pub struct PipeBuffer {
	/// The buffer's buffer.
	buffer: RingBuffer<u8, Vec<u8>>,

	/// The number of readers on the pipe.
	readers: usize,
	/// The number of writers on the pipe.
	writers: usize,

	/// The queue of processing waiting to read from the pipe.
	rd_queue: WaitQueue,
	/// The queue of processing waiting to write to the pipe.
	wr_queue: WaitQueue,
}

impl PipeBuffer {
	/// Returns the length of the data to be read in the buffer.
	pub fn get_data_len(&self) -> usize {
		self.buffer.get_data_len()
	}

	/// Returns the available space in the buffer in bytes.
	pub fn get_available_len(&self) -> usize {
		self.buffer.get_available_len()
	}
}

impl TryDefault for PipeBuffer {
	fn try_default() -> Result<Self, Self::Error> {
		Ok(Self {
			buffer: RingBuffer::new(vec![0; limits::PIPE_BUF]?),

			readers: 0,
			writers: 0,

			rd_queue: WaitQueue::default(),
			wr_queue: WaitQueue::default(),
		})
	}
}

impl Buffer for PipeBuffer {
	fn get_capacity(&self) -> usize {
		self.buffer.get_size()
	}

	fn increment_open(&mut self, read: bool, write: bool) {
		if read {
			self.readers += 1;
		}
		if write {
			self.writers += 1;
		}
	}

	fn decrement_open(&mut self, read: bool, write: bool) {
		if read {
			self.readers -= 1;
		}
		if write {
			self.writers -= 1;
		}
		if (self.readers == 0) != (self.writers == 0) {
			self.rd_queue.wake_all();
			self.wr_queue.wake_all();
		}
	}
}

impl NodeOps for PipeBuffer {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Fifo,
			mode: 0o666,
			..Default::default()
		})
	}

	fn ioctl(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		request: ioctl::Request,
		argp: *const c_void,
	) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::FIONREAD => {
				let count_ptr = SyscallPtr::<c_int>::from_syscall_arg(argp as usize);
				count_ptr.copy_to_user(self.get_available_len() as _)?;
			}
			_ => return Err(errno!(ENOTTY)),
		}
		Ok(0)
	}

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		buf: &mut [u8],
	) -> EResult<usize> {
		let len = self.buffer.read(buf);
		if len > 0 {
			self.wr_queue.wake_next();
		}
		Ok(len)
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		buf: &[u8],
	) -> EResult<usize> {
		if unlikely(buf.len() == 0) {
			return Ok(0);
		}
		if self.readers == 0 {
			Process::current().lock().kill(Signal::SIGPIPE);
			return Err(errno!(EPIPE));
		}
		let len = self.buffer.write(buf);
		self.rd_queue.wake_next();
		Ok(len)
	}
}
