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

//! This module implements a procfs node which allows to get the list of
//! mountpoint.

use crate::{
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		mountpoint,
		perm::{Gid, Uid},
		Mode,
	},
	process::{pid::Pid, Process},
};
use core::cmp::min;
use utils::{collections::string::String, errno, errno::EResult, format, io::IO};

/// Structure representing the mounts node of the procfs.
#[derive(Debug)]
pub struct Mounts {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Mounts {
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

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(FileContent::Regular.into())
	}
}

impl IO for Mounts {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		if buff.is_empty() {
			return Ok((0, false));
		}

		// Generate content
		let mut content = String::new();
		let mountpoints = mountpoint::MOUNT_POINTS.lock();

		for (_, mp_mutex) in mountpoints.iter() {
			let mp = mp_mutex.lock();

			let fs_type = mp.get_filesystem_type();
			let flags = "TODO"; // TODO

			let s = format!(
				"{source} {target} {fs_type} {flags} 0 0\n",
				source = mp.get_source(),
				target = mp.get_target_path(),
			)?;
			content.push_str(s)?;
		}

		// Copying content to userspace buffer
		let content_bytes = content.as_bytes();
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> EResult<u64> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
