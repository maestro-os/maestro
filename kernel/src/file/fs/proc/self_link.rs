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

//! Implementation of the `self` symlink, which points to the current process's directory.

use crate::{
	file::{fs::NodeOps, vfs::node::Node},
	format_content,
	process::{mem_space::copy::UserSlice, Process},
};
use utils::errno::EResult;

/// The `self` symlink.
#[derive(Debug, Default)]
pub struct SelfNode;

impl NodeOps for SelfNode {
	fn readlink(&self, _node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		let pid = Process::current().get_pid();
		format_content!(0, buf, "{pid}")
	}
}
