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

//! This module implements the `cwd` node, which is a link to the current
//! working directory of the process.

use crate::{
	errno::{EResult, Errno},
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		perm::{Gid, Uid},
		FileContent, Mode,
	},
	process::{pid::Pid, Process},
	util::{io::IO, TryClone},
};

/// Structure representing the `cwd` node.
#[derive(Debug)]
pub struct Cwd {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Cwd {
	fn get_mode(&self) -> Mode {
		0o777
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

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		let content = Process::get_by_pid(self.pid)
			.map(|mutex| {
				let proc = mutex.lock();
				proc.cwd.0.try_clone()
			})
			.transpose()?
			.unwrap_or_default();
		Ok(KernFSContent::Dynamic(FileContent::Link(content)))
	}
}

impl IO for Cwd {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		Err(errno!(EINVAL))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Err(errno!(EINVAL))
	}
}