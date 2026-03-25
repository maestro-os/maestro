/*
 * Copyright 2026 Luc Lenôtre
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

//! A framebuffer is a portion of RAM containing a bitmap that drives a video display

use crate::{
	file::{File, fs::FileOps},
	memory::{PhysAddr, VirtAddr, user::UserSlice},
	multiboot::FramebufferInfo,
};
use core::{hint::unlikely, slice};
use utils::{errno, errno::EResult};

/// A framebuffer
#[derive(Debug)]
pub struct Framebuffer(FramebufferInfo);

impl Framebuffer {
	/// Creates a new instance
	///
	/// If the framebuffer is outside reachable memory, the function returns `None`.
	pub fn new(info: FramebufferInfo) -> Option<Self> {
		let s = Self(info);
		// If the framebuffer is outside reachable memory, stop
		let physaddr: usize = s.0.framebuffer_addr.try_into().ok()?;
		physaddr.checked_add(s.len())?;
		// TODO remap with write-through
		Some(s)
	}

	/// Returns the virtual address to the beginning of the framebuffer
	pub fn addr(&self) -> VirtAddr {
		PhysAddr(self.0.framebuffer_addr as _)
			.kernel_to_virtual()
			.unwrap()
	}

	/// Returns the length of the buffer in bytes
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.0.framebuffer_pitch as usize * self.0.framebuffer_height as usize
	}
}

/// A framebuffer device
#[derive(Debug)]
pub struct FramebufferDev(Framebuffer);

impl FileOps for FramebufferDev {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let fb_len = self.0.len();
		let oob = off.checked_add(buf.len()).is_none_or(|l| l < fb_len);
		if unlikely(oob) {
			return Err(errno!(EINVAL));
		}
		let fb_slice = unsafe {
			let ptr = self.0.addr().as_ptr::<u8>().add(off);
			let len = buf.len() - off;
			slice::from_raw_parts_mut(ptr, len)
		};
		buf.copy_to_user(0, fb_slice)
	}

	fn write(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let fb_len = self.0.len();
		let oob = off.checked_add(buf.len()).is_none_or(|l| l < fb_len);
		if unlikely(oob) {
			return Err(errno!(EINVAL));
		}
		let fb_slice = unsafe {
			let ptr = self.0.addr().as_ptr::<u8>().add(off);
			let len = buf.len() - off;
			slice::from_raw_parts_mut(ptr, len)
		};
		buf.copy_from_user(0, fb_slice)
	}
}

// TODO on drop, restore memory remap
