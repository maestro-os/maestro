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

use crate::{
	device::BlockDeviceOps,
	memory::{buddy::ZONE_KERNEL, RcFrame},
	sync::mutex::Mutex,
};
use utils::{collections::btreemap::BTreeMap, errno::EResult, math::pow2, range_cmp};

/// A page cache
#[derive(Debug, Default)]
pub struct PageCache {
	/// Cached frames
	///
	/// The key is the file offset, in pages, to the start of the frame
	pages: Mutex<BTreeMap<u64, RcFrame>>,
}

impl PageCache {
	/// Looks for the frame containing the page at the offset `off`, or reads it from `ops`, then
	/// inserts it in the cache.
	///
	/// If the page is not in cache, the function returns `None`.
	pub fn get_or_insert(&self, off: u64, ops: &dyn BlockDeviceOps) -> EResult<RcFrame> {
		let mut pages = self.pages.lock();
		// First check cache
		let page = pages
			.cmp_get(|_, frame| range_cmp(frame.file_offset(), pow2(frame.order()) as u64, off));
		if let Some(page) = page {
			return Ok(page.clone());
		}
		// Cache miss: read and insert
		let frame = RcFrame::new(0, ZONE_KERNEL, off)?;
		ops.read_frame(&frame)?;
		pages.insert(off, frame.clone())?;
		Ok(frame)
	}
}
