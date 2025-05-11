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

//! Implementation of the `cwd` node, which is a link to the current
//! working directory of the process.

use crate::{
	file::{fs::NodeOps, vfs, vfs::node::Node},
	format_content,
	memory::user::UserSlice,
	process::{Process, pid::Pid},
};
use utils::{errno, errno::EResult};

/// The `cwd` node.
#[derive(Debug)]
pub struct Cwd(pub Pid);

impl NodeOps for Cwd {
	fn readlink(&self, _node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let fs = proc.fs.lock();
		let cwd = vfs::Entry::get_path(&fs.cwd)?;
		format_content!(0, buf, "{cwd}")
	}
}
