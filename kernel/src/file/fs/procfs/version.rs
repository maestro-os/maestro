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

//! The `/proc/version` file returns the version of the kernel.

use crate::file::fs::kernfs::{content::KernFSContent, node::KernFSNode};
use core::cmp::min;
use utils::{errno, errno::EResult, format, io::IO};

/// Structure representing the version node.
#[derive(Debug)]
pub struct Version {}

impl KernFSNode for Version {
	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(KernFSContent::Dynamic(FileContent::Regular))
	}
}

impl IO for Version {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		// TODO const format
		let version = format!("{} version {}\n", crate::NAME, crate::VERSION)?;
		let version_bytes = version.as_bytes();

		// Copy content to userspace buffer
		let len = min((version_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&version_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= version_bytes.len() as u64;
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
