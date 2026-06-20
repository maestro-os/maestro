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
	memory::{
		PhysAddr, VirtAddr,
		user::{UserPtr, UserSlice},
		vmem::KERNEL_VMEM,
	},
	multiboot::FramebufferInfo,
	syscall::{FromSyscallArg, ioctl},
};
use core::{
	ffi::{c_ulong, c_void},
	hint::unlikely,
	mem::ManuallyDrop,
};
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

/// Packed RGB pixel: framebuffer memory holds true color values.
const FB_VISUAL_TRUECOLOR: u32 = 2;
/// Packed pixels framebuffer type.
const FB_TYPE_PACKED_PIXELS: u32 = 0;

/// Description of a bitfield inside a pixel, as exposed by [`FbVarScreeninfo`].
#[repr(C)]
#[derive(Clone, Debug, Default)]
struct FbBitfield {
	/// Beginning of the bitfield, in bits.
	offset: u32,
	/// Length of the bitfield, in bits.
	length: u32,
	/// Whether the most significant bit is on the right.
	msb_right: u32,
}

/// Fixed screen information, as returned by [`ioctl::FBIOGET_FSCREENINFO`].
///
/// This mirrors the Linux `struct fb_fix_screeninfo`.
#[repr(C)]
#[derive(Clone, Debug, Default)]
struct FbFixScreeninfo {
	/// Identification string
	id: [u8; 16],
	/// Start of the framebuffer memory (physical address)
	smem_start: c_ulong,
	/// Length of the framebuffer memory, in bytes
	smem_len: u32,
	/// Framebuffer type (see `FB_TYPE_*`)
	type_: u32,
	/// Interleave for interleaved planes
	type_aux: u32,
	/// Visual type (see `FB_VISUAL_*`)
	visual: u32,
	/// Horizontal panning step, `0` if no hardware panning
	xpanstep: u16,
	/// Vertical panning step, `0` if no hardware panning
	ypanstep: u16,
	/// Vertical wrap step, `0` if no hardware ywrap
	ywrapstep: u16,
	/// Length of a line, in bytes
	line_length: u32,
	/// Start of memory mapped I/O (physical address)
	mmio_start: c_ulong,
	/// Length of memory mapped I/O, in bytes
	mmio_len: u32,
	/// Acceleration chip/card identifier
	accel: u32,
	/// Capabilities (see `FB_CAP_*`)
	capabilities: u16,
	/// Reserved for future compatibility
	reserved: [u16; 2],
}

/// Variable screen information, as returned by [`ioctl::FBIOGET_VSCREENINFO`].
///
/// This mirrors the Linux `struct fb_var_screeninfo`.
#[repr(C)]
#[derive(Clone, Debug, Default)]
struct FbVarScreeninfo {
	/// Visible horizontal resolution, in pixels
	xres: u32,
	/// Visible vertical resolution, in pixels
	yres: u32,
	/// Virtual horizontal resolution, in pixels
	xres_virtual: u32,
	/// Virtual vertical resolution, in pixels
	yres_virtual: u32,
	/// Horizontal offset from virtual to visible resolution
	xoffset: u32,
	/// Vertical offset from virtual to visible resolution
	yoffset: u32,
	/// Bits per pixel
	bits_per_pixel: u32,
	/// `0` for color, `1` for grayscale, `>1` for FOURCC
	grayscale: u32,
	/// Red bitfield in framebuffer memory
	red: FbBitfield,
	/// Green bitfield in framebuffer memory
	green: FbBitfield,
	/// Blue bitfield in framebuffer memory
	blue: FbBitfield,
	/// Transparency bitfield in framebuffer memory
	transp: FbBitfield,
	/// Non-zero for a non-standard pixel format
	nonstd: u32,
	/// Activation flags (see `FB_ACTIVATE_*`)
	activate: u32,
	/// Height of the picture, in mm
	height: u32,
	/// Width of the picture, in mm
	width: u32,
	/// Acceleration flags (obsolete)
	accel_flags: u32,
	/// Pixel clock, in picoseconds
	pixclock: u32,
	/// Time from sync to picture, in pixel clocks
	left_margin: u32,
	/// Time from picture to sync, in pixel clocks
	right_margin: u32,
	/// Time from sync to picture, in pixel clocks
	upper_margin: u32,
	/// Time from picture to sync, in pixel clocks
	lower_margin: u32,
	/// Length of the horizontal sync, in pixel clocks
	hsync_len: u32,
	/// Length of the vertical sync, in pixel clocks
	vsync_len: u32,
	/// Sync flags (see `FB_SYNC_*`)
	sync: u32,
	/// Video mode flags (see `FB_VMODE_*`)
	vmode: u32,
	/// Rotation angle, counter clockwise
	rotate: u32,
	/// Colorspace for FOURCC-based modes
	colorspace: u32,
	/// Reserved for future compatibility
	reserved: [u32; 4],
}

/// A framebuffer device
#[derive(Debug)]
pub struct FramebufferDev(Arc<Framebuffer>);

impl FramebufferDev {
	/// Builds the fixed screen information for the framebuffer.
	fn fix_screeninfo(&self) -> FbFixScreeninfo {
		let fb = self.0.info();
		let mut id = [0u8; 16];
		let name = b"maestro";
		id[..name.len()].copy_from_slice(name);
		FbFixScreeninfo {
			id,
			smem_start: fb.framebuffer_addr as _,
			smem_len: self.0.len() as _,
			type_: FB_TYPE_PACKED_PIXELS,
			visual: FB_VISUAL_TRUECOLOR,
			line_length: fb.framebuffer_pitch,
			..Default::default()
		}
	}

	/// Builds the variable screen information for the framebuffer.
	fn var_screeninfo(&self) -> FbVarScreeninfo {
		let fb = self.0.info();
		let rgb = &fb.framebuffer_rgb;
		FbVarScreeninfo {
			xres: fb.framebuffer_width,
			yres: fb.framebuffer_height,
			xres_virtual: fb.framebuffer_width,
			yres_virtual: fb.framebuffer_height,
			bits_per_pixel: fb.framebuffer_bpp as _,
			red: FbBitfield {
				offset: rgb.framebuffer_red_field_position as _,
				length: rgb.framebuffer_red_mask_size as _,
				msb_right: 0,
			},
			green: FbBitfield {
				offset: rgb.framebuffer_green_field_position as _,
				length: rgb.framebuffer_green_mask_size as _,
				msb_right: 0,
			},
			blue: FbBitfield {
				offset: rgb.framebuffer_blue_field_position as _,
				length: rgb.framebuffer_blue_mask_size as _,
				msb_right: 0,
			},
			..Default::default()
		}
	}
}

impl FileOps for FramebufferDev {
	fn ioctl(&self, _file: &File, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		match request.get_old_format() {
			ioctl::FBIOGET_FSCREENINFO => {
				let ptr = UserPtr::<FbFixScreeninfo>::from_ptr(argp as usize);
				ptr.copy_to_user(&self.fix_screeninfo())?;
				Ok(0)
			}
			ioctl::FBIOGET_VSCREENINFO => {
				let ptr = UserPtr::<FbVarScreeninfo>::from_ptr(argp as usize);
				ptr.copy_to_user(&self.var_screeninfo())?;
				Ok(0)
			}
			ioctl::FBIOPUT_VSCREENINFO => {
				// The framebuffer mode is fixed by the bootloader and cannot be reprogrammed
				let ptr = UserPtr::<FbVarScreeninfo>::from_ptr(argp as usize);
				let req = ptr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
				let cur = self.var_screeninfo();
				if unlikely(
					req.xres != cur.xres
						|| req.yres != cur.yres
						|| req.bits_per_pixel != cur.bits_per_pixel,
				) {
					return Err(errno!(EINVAL));
				}
				Ok(0)
			}
			ioctl::FBIOPUTCMAP => {
				// Ignored
				Ok(0)
			}
			_ => Err(errno!(EINVAL)),
		}
	}

	fn mmap_page(&self, _file: &File, offset: usize) -> EResult<Option<PhysAddr>> {
		// Bounds check
		let byte_off = offset
			.checked_mul(PAGE_SIZE)
			.ok_or_else(|| errno!(EINVAL))?;
		if unlikely(byte_off >= self.0.len()) {
			return Err(errno!(EINVAL));
		}
		// Return framebuffer physical address
		let base: usize = self.0.info().framebuffer_addr as usize;
		let phys_addr = PhysAddr(base.checked_add(byte_off).ok_or_else(|| errno!(EINVAL))?);
		Ok(Some(phys_addr))
	}

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
