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

//! The page cache allows to avoid unnecessary disk I/O by using all the available memory on the
//! system to cache the content of the disk.

use crate::{
	device::BlockDeviceOps,
	memory::{buddy::FrameOrder, RcFrame},
	sync::mutex::Mutex,
};
use utils::{collections::btreemap::BTreeMap, errno::EResult};

/// A page cache
#[derive(Debug, Default)]
pub struct PageCache {
	/// Cached frames
	///
	/// The key is the file offset, in pages, to the start of the frame
	frames: Mutex<BTreeMap<u64, RcFrame>>,
}

impl PageCache {
	/// Looks for the frame containing the page at the offset `off`, or reads it from `ops`, then
	/// inserts it in the cache.
	///
	/// If the page is not in cache, the function returns `None`.
	pub fn get_or_insert(
		&self,
		off: u64,
		order: FrameOrder,
		ops: &dyn BlockDeviceOps,
	) -> EResult<RcFrame> {
		let mut frames = self.frames.lock();
		match frames.get(&off) {
			// Cache hit
			Some(frame) if frame.order() == order => Ok(frame.clone()),
			// Cache miss: read and insert
			_ => {
				let frame = ops.read_frame(off, order)?;
				frames.insert(off, frame.clone())?;
				Ok(frame)
			}
		}
	}
}
