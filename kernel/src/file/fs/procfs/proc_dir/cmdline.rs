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

//! The cmdline node allows to retrieve the list of command line arguments of
//! the process.

use crate::{
	file::{
		fs::kernfs::node::{content_chunks, KernFSNode},
		perm::{Gid, Uid},
		FileType, Mode,
	},
	process::{pid::Pid, Process},
};
use utils::{errno, errno::EResult, io::IO};

/// Structure representing the cmdline node of the procfs.
#[derive(Debug)]
pub struct Cmdline {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Cmdline {
	fn get_file_type(&self) -> FileType {
		FileType::Regular
	}

	fn get_mode(&self) -> Mode {
		0o444
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

impl IO for Cmdline {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		let proc_mutex = Process::get_by_pid(self.pid).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();
		let iter = proc.argv.iter().flat_map(|s| [Ok(s.as_bytes()), Ok(b"\0")]);
		content_chunks(offset, buff, iter)
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> EResult<u64> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
