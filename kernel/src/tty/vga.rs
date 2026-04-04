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

//! VGA text mode implementation, allowing to easily write text on thescreen.
//!
//! This module doesn't support concurrency. It is the callers' reponsibility to
//! handle it.
//!
//! Note: The VGA text mode runs only when booting with a Legacy BIOS.

use crate::{
	arch::x86::io::{inb, outb},
	memory::PhysAddr,
	tty::Rgb,
};

/// Type representing a VGA text mode character.
pub type Char = u16;
/// Type representing a VGA text mode color.
pub type Color = u8;

/// Physical address of the VGA text buffer.
pub const BUFFER_PHYS: PhysAddr = PhysAddr(0xb8000);

/// Width of the screen in characters under the VGA text mode.
pub const WIDTH: u16 = 80;
/// Height of the screen in characters under the VGA text mode.
pub const HEIGHT: u16 = 25;

/// Width of the screen in pixels under the VGA text mode.
pub const PIXEL_WIDTH: u32 = 640;
/// Height of the screen in pixels under the VGA text mode.
pub const PIXEL_HEIGHT: u32 = 480;

/// The beginning scanline for the cursor.
pub const CURSOR_START: u8 = 0;
/// The ending scanline for the cursor.
pub const CURSOR_END: u8 = 15;

/// Returns a pointer to the VGA text buffer.
#[inline]
pub fn text_buf() -> *mut Char {
	BUFFER_PHYS.kernel_to_virtual().unwrap().as_ptr()
}

/// Converts an RGB color to the nearest VGA color
pub fn nearest_color((r, g, b): Rgb) -> Color {
	let round = |v: u8| -> u8 {
		if v < 43 {
			0
		} else if v < 128 {
			85
		} else if v < 213 {
			170
		} else {
			255
		}
	};
	let (r, g, b) = (round(r), round(g), round(b));
	// Brown
	if r == 170 && g == 85 && b == 0 {
		return 0x6;
	}
	let bright = r == 85 || r == 255 || g == 85 || g == 255 || b == 85 || b == 255;
	let b2 = r >= 128;
	let b1 = g >= 128;
	let b0 = b >= 128;
	(bright as Color) << 3 | (b2 as Color) << 2 | (b1 as Color) << 1 | (b0 as Color)
}

/// Enables the VGA text mode cursor.
pub fn enable_cursor() {
	unsafe {
		outb(0x3d4, 0x0a);
		outb(0x3d5, (inb(0x3d5) & 0xc0) | CURSOR_START);
		outb(0x3d4, 0x0b);
		outb(0x3d5, (inb(0x3d5) & 0xe0) | CURSOR_END);
	}
}

/// Disables the VGA text mode cursor.
pub fn disable_cursor() {
	unsafe {
		outb(0x3d4, 0x0a);
		outb(0x3d5, 0x20);
	}
}

/// Moves the VGA text mode cursor to the given position.
pub fn move_cursor(x: u16, y: u16) {
	let pos = y * WIDTH + x;
	unsafe {
		outb(0x3d4, 0x0f);
		outb(0x3d5, pos as u8);
		outb(0x3d4, 0x0e);
		outb(0x3d5, (pos >> 8) as u8);
	}
}
