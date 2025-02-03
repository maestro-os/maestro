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

//! TODO doc

use crate::{
	device::DeviceIO,
	file::FileLocation,
	memory::buddy,
	sync::{mutex::Mutex, once::OnceInit},
};
use core::{
	ptr::NonNull,
	sync::atomic::{Ordering::Acquire},
};
use utils::{
	collections::lru::LruCache,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};
use crate::file::vfs::node::Node;
use crate::memory::buddy::PageState;

/// Wrapper around a page to sync with the disk on drop.
pub struct MappedPage(&'static PageState);

impl MappedPage {
	pub fn as_ptr(&self) -> NonNull<[u8; PAGE_SIZE]> {
		self.0.as_ptr()
	}
	
	/// Synchronizes the page to the backing storage.
	/// 
	/// If there is no backing node, the function does nothing.
	pub fn sync(&self) -> EResult<()> {
		let Some(node) = &self.0.node else {
			return Ok(());
		};
		// TODO write content to node without caching
	}
}

impl Drop for MappedPage {
	fn drop(&mut self) {
		// If dirty, sync to disk
		if self.0.dirty.load(Acquire) {
			// TODO on error, warning?
			let _ = self.sync();
		}
		// Release reference to the node
		self.0.node = None;
		// Free page
		let ptr = self.as_ptr();
		unsafe {
			buddy::free_kernel(ptr.cast().as_ptr(), 0);
		}
	}
}

/// TODO doc
static LRU: OnceInit<Mutex<LruCache<(FileLocation, u64), Arc<MappedPage>>>> =
	unsafe { OnceInit::new() };

pub(crate) fn init() -> AllocResult<()> {
	let lru = Mutex::new(LruCache::new()?);
	unsafe {
		OnceInit::init(&LRU, lru);
	}
	Ok(())
}

// TODO need to pass both the device offset and file offset?
pub fn get(node: Arc<Node>, loc: FileLocation, off: u64) -> EResult<Arc<MappedPage>> {
	let mut lru = LRU.lock();
	if let Some(page) = lru.get(&(loc.clone(), off)) {
		return Ok(page.clone());
	}
	// Cache miss. Allocate a page and read the content from the device
	// TODO if allocations fails, shrink caches to reclaim memory, then retry
	let page = buddy::alloc(0)?;
	// TODO get page state, set node, offset and dirty=false
	// TODO need to map the page before we can populate it
	// Read content from the node
	let blk_off = off / dev.block_size().get();
	// Safe because no one else is using the page as it was just allocated
	let slice = unsafe { &mut *page.page.as_ptr() };
	dev.read(blk_off, slice)?;
	// Insert the page in cache
	lru.push((loc, off), page.clone())?;
	Ok(page)
}

pub fn shrink() {
	// TODO remove the last entry from LRU
}
