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
	/// Looks for a frame in cache at offset `off`, or reads it from `init` and inserts it in the
	/// cache.
	pub fn get_or_insert<Init: FnOnce() -> EResult<RcFrame>>(
		&self,
		off: u64,
		order: FrameOrder,
		init: Init,
	) -> EResult<RcFrame> {
		let mut frames = self.frames.lock();
		match frames.get(&off) {
			// Cache hit
			Some(frame) if frame.order() == order => Ok(frame.clone()),
			// Cache miss: read and insert
			_ => {
				let frame = init()?;
				frames.insert(off, frame.clone())?;
				Ok(frame)
			}
		}
	}
}
