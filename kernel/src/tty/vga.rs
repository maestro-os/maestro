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

/// VGA text mode color: Black
pub const COLOR_BLACK: Color = 0x0;
/// VGA text mode color: Blue
pub const COLOR_BLUE: Color = 0x1;
/// VGA text mode color: Green
pub const COLOR_GREEN: Color = 0x2;
/// VGA text mode color: Cyan
pub const COLOR_CYAN: Color = 0x3;
/// VGA text mode color: Red
pub const COLOR_RED: Color = 0x4;
/// VGA text mode color: Magenta
pub const COLOR_MAGENTA: Color = 0x5;
/// VGA text mode color: Brown
pub const COLOR_BROWN: Color = 0x6;
/// VGA text mode color: Light Grey
pub const COLOR_LIGHT_GREY: Color = 0x7;
/// VGA text mode color: Dark Grey
pub const COLOR_DARK_GREY: Color = 0x8;
/// VGA text mode color: Light Blue
pub const COLOR_LIGHT_BLUE: Color = 0x9;
/// VGA text mode color: Light Green
pub const COLOR_LIGHT_GREEN: Color = 0xa;
/// VGA text mode color: Light Cyan
pub const COLOR_LIGHT_CYAN: Color = 0xb;
/// VGA text mode color: Light Red
pub const COLOR_LIGHT_RED: Color = 0xc;
/// VGA text mode color: Light Magenta
pub const COLOR_LIGHT_MAGENTA: Color = 0xd;
/// VGA text mode color: Yellow
pub const COLOR_YELLOW: Color = 0xe;
/// VGA text mode color: White
pub const COLOR_WHITE: Color = 0xf;

/// VGA text mode default color
pub const DEFAULT_COLOR: Color = COLOR_WHITE | (COLOR_BLACK << 4);

/// The beginning scanline for the cursor.
pub const CURSOR_START: u8 = 0;
/// The ending scanline for the cursor.
pub const CURSOR_END: u8 = 15;

/// Returns the virtual address of the VGA text buffer.
#[inline]
pub fn get_buffer_virt() -> *mut Char {
	BUFFER_PHYS.kernel_to_virtual().unwrap().as_ptr()
}

/// Returns the value for the given foreground color `fg` and background color
/// `bg`.
#[inline]
pub fn entry_color(fg: Color, bg: Color) -> Color {
	fg | (bg << 4)
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

/// Returns the current position of the cursor.
pub fn get_cursor_position() -> (u16, u16) {
	let mut pos: u16 = 0;
	unsafe {
		outb(0x3d4, 0x0f);
		pos |= inb(0x3d5) as u16;
		outb(0x3d4, 0x0e);
		pos |= (inb(0x3d5) as u16) << 8;
	}
	(pos % WIDTH, pos / WIDTH)
}

/// Moves the VGA text mode cursor to the given position.
pub fn move_cursor(x: u16, y: u16) {
	let pos = y * WIDTH + x;
	unsafe {
		outb(0x3d4, 0x0f);
		outb(0x3d5, (pos & 0xff) as u8);
		outb(0x3d4, 0x0e);
		outb(0x3d5, ((pos >> 8) & 0xff) as u8);
	}
}
