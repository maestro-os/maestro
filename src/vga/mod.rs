//! This file handles the VGA text mode, allowing to easily write text on the
//! screen.
//! This module doesn't support concurrency. It is the callers' reponsibility to handle it.
//!
//! Note: The VGA text mode runs only when booting with a Legacy BIOS.

use crate::io;
use crate::memory::vmem;
use crate::memory;

/// Type representing a VGA text mode character.
pub type Char = u16;
/// Type representing a VGA text mode color.
pub type Color = u8;
/// Type representing a VGA text mode position.
pub type Pos = i16;

/// Physical address of the VGA text buffer.
pub const BUFFER_PHYS: *mut Char = 0xb8000 as _;

/// Width of the screen in characters under the VGA text mode.
pub const WIDTH: Pos = 80;
/// Height of the screen in characters under the VGA text mode.
pub const HEIGHT: Pos = 25;
/// The size in bytes of the VGA text buffer.
pub const BUFFER_SIZE: u32 = (WIDTH * HEIGHT * core::mem::size_of::<i16>() as Pos) as u32;

/// VGA text mode color: Black
pub const COLOR_BLACK: Color			= 0x0;
/// VGA text mode color: Blue
pub const COLOR_BLUE: Color			    = 0x1;
/// VGA text mode color: Green
pub const COLOR_GREEN: Color			= 0x2;
/// VGA text mode color: Cyan
pub const COLOR_CYAN: Color			    = 0x3;
/// VGA text mode color: Red
pub const COLOR_RED: Color			    = 0x4;
/// VGA text mode color: Magenta
pub const COLOR_MAGENTA: Color		    = 0x5;
/// VGA text mode color: Brown
pub const COLOR_BROWN: Color			= 0x6;
/// VGA text mode color: Light Grey
pub const COLOR_LIGHT_GREY: Color	    = 0x7;
/// VGA text mode color: Dark Grey
pub const COLOR_DARK_GREY: Color		= 0x8;
/// VGA text mode color: Light Blue
pub const COLOR_LIGHT_BLUE: Color	    = 0x9;
/// VGA text mode color: Light Green
pub const COLOR_LIGHT_GREEN: Color	    = 0xa;
/// VGA text mode color: Light Cyan
pub const COLOR_LIGHT_CYAN: Color	    = 0xb;
/// VGA text mode color: Light Red
pub const COLOR_LIGHT_RED: Color		= 0xc;
/// VGA text mode color: Light Magenta
pub const COLOR_LIGHT_MAGENTA: Color	= 0xd;
/// VGA text mode color: Yellow
pub const COLOR_YELLOW: Color		    = 0xe;
/// VGA text mode color: White
pub const COLOR_WHITE: Color			= 0xf;

/// VGA text mode default color.
pub const DEFAULT_COLOR: Color = COLOR_WHITE | (COLOR_BLACK << 4);

/// The beginning scanline for the cursor.
pub const CURSOR_START: u8 = 0;
/// The ending scanline for the cursor.
pub const CURSOR_END: u8 = 15;

/// Returns the virtual address of the VGA text buffer.
#[inline]
pub fn get_buffer_virt() -> *mut Char {
	(memory::PROCESS_END as usize + BUFFER_PHYS as usize) as _
}

/// Returns the value for the given foreground color `fg` and background color `bg`.
#[inline]
pub fn entry_color(fg: Color, bg: Color) -> Color {
	fg | (bg << 4)
}

/// Clears the VGA text buffer.
pub fn clear() {
	for i in 0..(WIDTH * HEIGHT) {
		unsafe {
			vmem::write_lock_wrap(|| {
				*get_buffer_virt().offset(i as isize) = (DEFAULT_COLOR as Char) << 8;
			});
		}
	}
}

/// Enables the VGA text mode cursor.
pub fn enable_cursor() {
	unsafe {
		io::outb(0x3d4, 0x0a);
		io::outb(0x3d5, (io::inb(0x3d5) & 0xc0) | CURSOR_START);
		io::outb(0x3d4, 0x0b);
		io::outb(0x3d5, (io::inb(0x3d5) & 0xe0) | CURSOR_END);
	}
}

/// Disables the VGA text mode cursor.
pub fn disable_cursor() {
	unsafe {
		io::outb(0x3d4, 0x0a);
		io::outb(0x3d5, 0x20);
	}
}

/// Returns the current position of the cursor.
pub fn get_cursor_position() -> (Pos, Pos) {
	let mut pos: u16 = 0;

	unsafe {
		io::outb(0x3d4, 0x0f);
		pos |= io::inb(0x3d5) as u16;
		io::outb(0x3d4, 0x0e);
		pos |= (io::inb(0x3d5) as u16) << 8;
	}

	(pos as i16 % WIDTH, pos as i16 / WIDTH)
}

/// Moves the VGA text mode cursor to the given position.
pub fn move_cursor(x: Pos, y: Pos) {
	let pos = y * WIDTH + x;

	unsafe {
		io::outb(0x3d4, 0x0f);
		io::outb(0x3d5, (pos & 0xff) as u8);
		io::outb(0x3d4, 0x0e);
		io::outb(0x3d5, ((pos >> 8) & 0xff) as u8);
	}
}

/// Writes the given character `c` at the given position `x`/`y` on the screen with the default
/// color.
pub fn putchar(c: char, x: Pos, y: Pos) {
	putchar_color(c, DEFAULT_COLOR, x, y);
}

/// Writes the given character `c` at the given position `x`/`y` on the screen
/// with the given color `color`.
pub fn putchar_color(c: char, color: Color, x: Pos, y: Pos) {
	debug_assert!(x >= 0);
	debug_assert!(x < WIDTH);
	debug_assert!(y >= 0);
	debug_assert!(y < HEIGHT);

	let pos = (y as usize) * (WIDTH as usize) + (x as usize);
	let c = (c as Char) | (color as Char) << 8;

	debug_assert!(pos < BUFFER_SIZE as usize);
	unsafe {
		vmem::write_lock_wrap(|| {
			*get_buffer_virt().offset(pos as isize) = c;
		});
	}
}
