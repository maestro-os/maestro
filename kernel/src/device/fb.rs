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
	arch::x86::paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_WRITE, FLAG_WRITE_THROUGH},
	device::{CharDev, DeviceID, DeviceType, id::MajorBlock, register_char},
	file::{File, fs::FileOps},
	memory::{PhysAddr, VirtAddr, user::UserSlice, vmem::KERNEL_VMEM},
	multiboot::FramebufferInfo,
};
use core::{hint::unlikely, mem::ManuallyDrop};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
};

/// Flags used to map a framebuffer
pub const MAP_FLAGS: usize = FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL;

/// A framebuffer
#[derive(Debug)]
pub struct Framebuffer(FramebufferInfo);

impl Framebuffer {
	fn remap(fb: &Self) -> Option<()> {
		// If the framebuffer is outside reachable memory, stop
		let physaddr: usize = fb.0.framebuffer_addr.try_into().ok()?;
		physaddr.checked_add(fb.len())?;
		// Remap
		let physaddr = PhysAddr(physaddr);
		let virtaddr = physaddr.kernel_to_virtual()?;
		KERNEL_VMEM.map_range(physaddr, virtaddr, fb.len().div_ceil(PAGE_SIZE), MAP_FLAGS);
		Some(())
	}

	/// Creates a new instance
	///
	/// If the framebuffer is outside reachable memory, the function returns `None`.
	pub fn new(info: FramebufferInfo) -> AllocResult<Option<Arc<Self>>> {
		let fb = Self(info);
		if Self::remap(&fb).is_some() {
			Ok(Some(Arc::new(fb)?))
		} else {
			Ok(None)
		}
	}

	/// Returns the framebuffer's info
	#[inline]
	pub fn info(&self) -> &FramebufferInfo {
		&self.0
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

// TODO undo memory remap on fb drop? (determine if this is useful)

/// A framebuffer device
#[derive(Debug)]
pub struct FramebufferDev(Arc<Framebuffer>);

impl FileOps for FramebufferDev {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let fb_len = self.0.len();
		let oob = off.checked_add(buf.len()).is_none_or(|l| l > fb_len);
		if unlikely(oob) {
			return Err(errno!(EINVAL));
		}
		unsafe {
			let ptr = self.0.addr().as_ptr::<u8>().add(off);
			buf.copy_to_user_raw(0, ptr, buf.len())
		}
	}

	fn write(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let fb_len = self.0.len();
		let oob = off.checked_add(buf.len()).is_none_or(|l| l > fb_len);
		if unlikely(oob) {
			return Err(errno!(EINVAL));
		}
		unsafe {
			let ptr = self.0.addr().as_ptr::<u8>().add(off);
			buf.copy_from_user_raw(0, ptr, buf.len())
		}
	}
}

/// Creates framebuffer device.
pub(crate) fn create(fb: Arc<Framebuffer>) -> EResult<()> {
	// TODO store somewhere for dynamic allocations when we have display hotplug
	let mut fb_major = ManuallyDrop::new(MajorBlock::new_fixed(DeviceType::Char, 29)?);
	let minor = fb_major.alloc_minor(None)?;
	register_char(CharDev::new(
		DeviceID {
			major: fb_major.get_major(),
			minor,
		},
		PathBuf::try_from(b"/dev/fb0")?,
		0o660,
		FramebufferDev(fb),
	)?)?;
	Ok(())
}
