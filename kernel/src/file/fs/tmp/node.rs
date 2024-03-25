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

//! This module implements regular file node for the tmpfs.

use crate::{
	file::{
		fs::{kernfs::content::KernFSContent, tmp::KernFSNode},
		perm::{Gid, Uid},
		FileContent, Mode,
	},
	time::{
		clock,
		clock::CLOCK_MONOTONIC,
		unit::{Timestamp, TimestampScale},
	},
};
use core::cmp::{max, min};
use utils::{collections::vec::Vec, errno, errno::EResult, io::IO};

/// Structure representing a regular file node in the tmpfs.
#[derive(Debug)]
pub struct TmpFSRegular {
	/// The number of hard links to the node.
	hard_links_count: u16,

	/// The directory's permissions.
	mode: Mode,
	/// The directory's owner user ID.
	uid: Uid,
	/// The directory's owner group ID.
	gid: Gid,

	/// Timestamp of the last modification of the metadata.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,

	/// The content of the file.
	content: Vec<u8>,
}

impl TmpFSRegular {
	/// Creates a new instance.
	pub fn new(mode: Mode, uid: Uid, gid: Gid) -> Self {
		// The current timestamp
		let ts = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);

		Self {
			hard_links_count: 1,

			mode,
			uid,
			gid,

			ctime: ts,
			mtime: ts,
			atime: ts,

			content: Vec::new(),
		}
	}
}

impl KernFSNode for TmpFSRegular {
	fn get_hard_links_count(&self) -> u16 {
		self.hard_links_count
	}

	fn set_hard_links_count(&mut self, hard_links_count: u16) {
		self.hard_links_count = hard_links_count;
	}

	fn get_mode(&self) -> Mode {
		self.mode
	}

	fn set_mode(&mut self, mode: Mode) {
		self.mode = mode;
	}

	fn get_uid(&self) -> Uid {
		self.uid
	}

	fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	fn get_gid(&self) -> Gid {
		self.gid
	}

	fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	fn get_atime(&self) -> Timestamp {
		self.atime
	}

	fn set_atime(&mut self, ts: Timestamp) {
		self.atime = ts;
	}

	fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	fn set_ctime(&mut self, ts: Timestamp) {
		self.ctime = ts;
	}

	fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	fn set_mtime(&mut self, ts: Timestamp) {
		self.mtime = ts;
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(FileContent::Regular.into())
	}
}

impl IO for TmpFSRegular {
	fn get_size(&self) -> u64 {
		self.content.len() as _
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		if offset > self.content.len() as u64 {
			return Err(errno!(EINVAL));
		}

		let off = offset as usize;
		let len = min(self.content.len() - off, buff.len());
		buff[..len].copy_from_slice(&self.content.as_slice()[off..(off + len)]);

		let eof = off + len >= self.content.len();
		Ok((len as _, eof))
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> EResult<u64> {
		if offset > self.content.len() as u64 {
			return Err(errno!(EINVAL));
		}

		let off = offset as usize;
		let new_len = max(off + buff.len(), self.content.len());
		self.content.resize(new_len, 0)?;

		self.content.as_mut_slice()[off..(off + buff.len())].copy_from_slice(buff);

		Ok(buff.len() as _)
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
