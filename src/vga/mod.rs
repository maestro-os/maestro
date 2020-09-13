use super::io as io;
use super::memory as memory;

/*
 * This file handles the VGA text mode, allowing to easily write text on the
 * screen.
 *
 * Note: The VGA text mode runs only when booting with a Legacy BIOS.
 */

// TODO Save enable/disable cursor state
// TODO Spinlock?

pub type Char = u16;
pub type Color = u8;
pub type Pos = i16;

/*
 * Physical address of the VGA text buffer.
 */
pub const BUFFER_PHYS: *mut Char = 0xb8000 as *mut Char;
/*
 * Virtual address of the VGA text buffer.
 */
pub const BUFFER_VIRT: *mut Char = unsafe { (memory::PROCESS_END as usize + BUFFER_PHYS as usize) as *mut Char };
pub const BUFFER: *mut Char = BUFFER_VIRT;

/*
 * Width of the screen in characters under the VGA text mode.
 */
pub const WIDTH: Pos = 80;
/*
 * Height of the screen in characters under the VGA text mode.
 */
pub const HEIGHT: Pos = 25;
/*
 * The size in bytes of the VGA text buffer.
 */
pub const BUFFER_SIZE: u32 = (WIDTH * HEIGHT * core::mem::size_of::<i16>() as Pos) as u32;

/*
 * VGA text mode colors.
 */
pub const COLOR_BLACK: Color			= 0x0;
pub const COLOR_BLUE: Color			    = 0x1;
pub const COLOR_GREEN: Color			= 0x2;
pub const COLOR_CYAN: Color			    = 0x3;
pub const COLOR_RED: Color			    = 0x4;
pub const COLOR_MAGENTA: Color		    = 0x5;
pub const COLOR_BROWN: Color			= 0x6;
pub const COLOR_LIGHT_GREY: Color	    = 0x7;
pub const COLOR_DARK_GREY: Color		= 0x8;
pub const COLOR_LIGHT_BLUE: Color	    = 0x9;
pub const COLOR_LIGHT_GREEN: Color	    = 0xa;
pub const COLOR_LIGHT_CYAN: Color	    = 0xb;
pub const COLOR_LIGHT_RED: Color		= 0xc;
pub const COLOR_LIGHT_MAGENTA: Color	= 0xd;
pub const COLOR_YELLOW: Color		    = 0xe;
pub const COLOR_WHITE: Color			= 0xf;

/*
 * VGA text mode default color.
 */
pub const DEFAULT_COLOR: Color = COLOR_WHITE | (COLOR_BLACK << 4);

pub const CURSOR_START: u8 = 0;
pub const CURSOR_END: u8 = 15;

/*
 * Returns the value for the given foreground color `fg` and background color `bg`.
 */
pub fn entry_color(fg: Color, bg: Color) -> Color {
	fg | (bg << 4)
}

/*
 * Clears the VGA text buffer.
 */
pub fn clear() {
	for i in 0..(WIDTH * HEIGHT) {
		unsafe {
			*BUFFER.offset(i as isize) = (DEFAULT_COLOR as Char) << 8;
		}
	}
}

/*
 * Enables the VGA text mode cursor.
 */
pub fn enable_cursor() {
	unsafe {
		io::outb(0x3d4, 0x0a);
		io::outb(0x3d5, (io::inb(0x3d5) & 0xc0) | CURSOR_START);
		io::outb(0x3d4, 0x0b);
		io::outb(0x3d5, (io::inb(0x3d5) & 0xe0) | CURSOR_END);
	}
}

/*
 * Disables the VGA text mode cursor.
 */
pub fn disable_cursor() {
	unsafe {
		io::outb(0x3d4, 0x0a);
		io::outb(0x3d5, 0x20);
	}
}

/*
 * Moves the VGA text mode cursor to the given position.
 */
pub fn move_cursor(x: Pos, y: Pos) {
	let pos = y * WIDTH + x;

	unsafe {
		io::outb(0x3d4, 0x0f);
		io::outb(0x3d5, (pos & 0xff) as u8);
		io::outb(0x3d4, 0x0e);
		io::outb(0x3d5, ((pos >> 8) & 0xff) as u8);
	}
}

/*
 * Writes the given character `c` at the given position `x`/`y` on the screen with the default color.
 */
pub fn putchar(c: char, x: Pos, y: Pos) {
	putchar_color(c, DEFAULT_COLOR, x, y);
}

/*
 * Writes the given character `c` at the given position `x`/`y` on the screen
 * with the given color `color`.
 */
pub fn putchar_color(c: char, color: Color, x: Pos, y: Pos) {
	assert!(x >= 0);
	assert!(x < WIDTH);
	assert!(y >= 0);
	assert!(y < HEIGHT);

	let pos = (y as usize) * (WIDTH as usize) + (x as usize);
	let c = (c as Char) | (color as Char) << 8;

	assert!(pos < BUFFER_SIZE as usize);
	unsafe {
		*BUFFER.offset(pos as isize) = c;
	}
}
