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
	file::buffer::BlockHandler,
	limits,
	process::{
		mem_space::{ptr::SyscallPtr, MemSpace},
		Process,
	},
	syscall::ioctl,
};
use core::ffi::{c_int, c_void};
use utils::{
	collections::{ring_buffer::RingBuffer, vec::Vec},
	errno,
	errno::EResult,
	io,
	io::IO,
	lock::IntMutex,
	ptr::arc::Arc,
	vec, TryDefault,
};

/// Structure representing a buffer buffer.
#[derive(Debug)]
pub struct PipeBuffer {
	/// The buffer's buffer.
	buffer: RingBuffer<u8, Vec<u8>>,

	/// The number of reading ends attached to the pipe.
	read_ends: u32,
	/// The number of writing ends attached to the pipe.
	write_ends: u32,

	/// The pipe's block handler.
	block_handler: BlockHandler,
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

			read_ends: 0,
			write_ends: 0,

			block_handler: BlockHandler::new(),
		})
	}
}

impl Buffer for PipeBuffer {
	fn get_capacity(&self) -> usize {
		self.buffer.get_size()
	}

	fn increment_open(&mut self, read: bool, write: bool) {
		if read {
			self.read_ends += 1;
		}

		if write {
			self.write_ends += 1;
		}
	}

	fn decrement_open(&mut self, read: bool, write: bool) {
		if read {
			self.read_ends -= 1;

			if self.read_ends == 0 {
				self.block_handler.wake_processes(io::POLLERR);
			}
		}

		if write {
			self.write_ends -= 1;

			if self.write_ends == 0 {
				self.block_handler.wake_processes(io::POLLERR);
			}
		}
	}

	fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> EResult<()> {
		self.block_handler.add_waiting_process(proc, mask)
	}

	fn ioctl(
		&mut self,
		mem_space: Arc<IntMutex<MemSpace>>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::FIONREAD => {
				let mut mem_space_guard = mem_space.lock();
				let count_ptr: SyscallPtr<c_int> = (argp as usize).into();
				let count_ref = count_ptr
					.get_mut(&mut mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?;
				*count_ref = self.get_available_len() as _;
			}

			_ => return Err(errno!(ENOTTY)),
		}

		Ok(0)
	}
}

impl IO for PipeBuffer {
	fn get_size(&self) -> u64 {
		self.get_data_len() as _
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, buf: &mut [u8]) -> EResult<(u64, bool)> {
		let len = self.buffer.read(buf);
		let eof = self.write_ends == 0 && self.get_data_len() == 0;

		self.block_handler.wake_processes(io::POLLOUT);

		Ok((len as _, eof))
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, buf: &[u8]) -> EResult<u64> {
		if self.read_ends > 0 {
			let len = self.buffer.write(buf);

			self.block_handler.wake_processes(io::POLLIN);

			Ok(len as _)
		} else {
			Err(errno!(EPIPE))
		}
	}

	fn poll(&mut self, mask: u32) -> EResult<u32> {
		let mut result = 0;

		if mask & io::POLLIN != 0 && self.get_data_len() > 0 {
			result |= io::POLLIN;
		}
		if mask & io::POLLOUT != 0 && self.get_available_len() > 0 {
			result |= io::POLLOUT;
		}
		if mask & io::POLLPRI != 0 && self.read_ends == 0 {
			result |= io::POLLPRI;
		}

		Ok(result)
	}
}
