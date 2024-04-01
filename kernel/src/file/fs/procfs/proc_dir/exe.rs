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

//! This module implements the `exe` node, which is a link to the executable
//! file of the process.

use crate::{
	file::{
		fs::kernfs::node::{content_chunks, KernFSNode},
		perm::{Gid, Uid},
		FileType, Mode,
	},
	process::{pid::Pid, Process},
};
use core::iter;
use utils::{errno, errno::EResult, io::IO};

/// Structure representing the `exe` node.
#[derive(Debug)]
pub struct Exe {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Exe {
	fn get_mode(&self) -> Mode {
		0o777
	}

	fn get_file_type(&self) -> FileType {
		FileType::Link
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			0
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			0
		}
	}
}

impl IO for Exe {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		let Some(proc) = Process::get_by_pid(self.pid) else {
			return content_chunks(offset, buff, iter::empty());
		};
		let proc = proc.lock();
		content_chunks(offset, buff, iter::once(Ok(proc.exec_path.as_bytes())))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> EResult<u64> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		Err(errno!(EINVAL))
	}
}
