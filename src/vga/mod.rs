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

pub type Color = u16;
pub type Pos = u16;

/*
 * Physical address of the VGA text buffer.
 */
pub const VGA_BUFFER_PHYS: *mut u16 = 0xb8000 as *mut u16;
/*
 * Virtual address of the VGA text buffer.
 */
pub const VGA_BUFFER_VIRT: *mut u16 = unsafe { (memory::PROCESS_END as usize + VGA_BUFFER_PHYS as usize) as *mut u16 };
pub const VGA_BUFFER: *mut u16 = VGA_BUFFER_VIRT;

/*
 * Width of the screen in characters under the VGA text mode.
 */
pub const VGA_WIDTH: Pos = 80;
/*
 * Height of the screen in characters under the VGA text mode.
 */
pub const VGA_HEIGHT: Pos = 25;
/*
 * The size in bytes of the VGA text buffer.
 */
pub const VGA_BUFFER_SIZE: u32 = (VGA_WIDTH * VGA_HEIGHT * core::mem::size_of::<i16>() as Pos) as u32;

/*
 * VGA text mode colors.
 */
pub const VGA_COLOR_BLACK: Color			= 0x0;
pub const VGA_COLOR_BLUE: Color			    = 0x1;
pub const VGA_COLOR_GREEN: Color			= 0x2;
pub const VGA_COLOR_CYAN: Color			    = 0x3;
pub const VGA_COLOR_RED: Color			    = 0x4;
pub const VGA_COLOR_MAGENTA: Color		    = 0x5;
pub const VGA_COLOR_BROWN: Color			= 0x6;
pub const VGA_COLOR_LIGHT_GREY: Color	    = 0x7;
pub const VGA_COLOR_DARK_GREY: Color		= 0x8;
pub const VGA_COLOR_LIGHT_BLUE: Color	    = 0x9;
pub const VGA_COLOR_LIGHT_GREEN: Color	    = 0xa;
pub const VGA_COLOR_LIGHT_CYAN: Color	    = 0xb;
pub const VGA_COLOR_LIGHT_RED: Color		= 0xc;
pub const VGA_COLOR_LIGHT_MAGENTA: Color	= 0xd;
pub const VGA_COLOR_YELLOW: Color		    = 0xe;
pub const VGA_COLOR_WHITE: Color			= 0xf;

/*
 * VGA text mode default color.
 */
pub const VGA_DEFAULT_COLOR: Color = VGA_COLOR_WHITE | (VGA_COLOR_BLACK << 4);

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
    for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
		unsafe {
			*VGA_BUFFER.offset(i as isize) = VGA_DEFAULT_COLOR << 8;
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
    let pos = y * VGA_WIDTH + x;

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
    putchar_color(c, VGA_DEFAULT_COLOR, x, y);
}

/*
 * Writes the given character `c` at the given position `x`/`y` on the screen
 * with the given color `color`.
 */
pub fn putchar_color(c: char, color: Color, x: Pos, y: Pos) {
	unsafe {
		*VGA_BUFFER.offset((y * VGA_WIDTH + x) as isize) = (c as u16) | (color as u16) << 8;
	}
}
