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

//! The `cmdline` node allows to retrieve the list of command line arguments of
//! the process.

use crate::{
	file::{
		fs::{proc::get_proc_owner, NodeOps},
		FileLocation, FileType, Stat,
	},
	format_content,
	process::{pid::Pid, Process},
};
use core::{fmt, fmt::Formatter};
use utils::{errno, errno::EResult};

struct CmdlineDisp<'p>(&'p Process);

impl<'p> fmt::Display for CmdlineDisp<'p> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let argv = self.0.argv.get();
		for a in argv.iter() {
			write!(f, "{a}\0")?;
		}
		Ok(())
	}
}

/// The cmdline node of the proc.
#[derive(Clone, Debug)]
pub struct Cmdline(Pid);

impl From<Pid> for Cmdline {
	fn from(pid: Pid) -> Self {
		Self(pid)
	}
}

impl NodeOps for Cmdline {
	fn get_stat(&self, _loc: &FileLocation) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			mode: FileType::Regular.to_mode() | 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read_content(&self, _loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		format_content!(off, buf, "{}", CmdlineDisp(&proc))
	}
}
