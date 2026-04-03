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

//! The TeleTypeWriter (TTY) is an electromechanical device that was used in the
//! past to send and receive typed messages through a communication channel.
//!
//! This module implements line discipline for TTYs.
//!
//! At startup, the kernel has one TTY: the init TTY, which is stored separately
//! because at the time of creation, memory management isn't initialized yet.

mod ansi;
pub mod termios;
pub mod vga;

use crate::{
	device::{fb, fb::Framebuffer},
	memory::{user::UserSlice, vmem::KERNEL_VMEM},
	multiboot::BootInfo,
	process::{Process, pid::Pid, signal::Signal},
	sync::{spin::IntSpin, wait_queue::WaitQueue},
	tty::{
		ansi::{ANSIBuffer, ESCAPE},
		termios::{Termios, consts::*},
	},
};
use core::{cmp::min, hint::unlikely, ptr};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, EResult},
	vec,
};

/// The number of history lines for one TTY.
const HISTORY_LINES: usize = 128;

/// Default text color
pub const DEFAULT_COLOR: vga::Color = vga::COLOR_WHITE | (vga::COLOR_BLACK << 4);

/// The size of a tabulation in space-equivalent.
const TAB_SIZE: usize = 4;

/// The maximum number of characters in the input buffer of a TTY.
const INPUT_MAX: usize = 4096;

// TODO Implement character size mask

/// Structure representing a window size for a terminal.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct WinSize {
	/// The number of rows.
	pub ws_row: u16,
	/// The number of columns.
	pub ws_col: u16,
	/// The width of the window in pixels.
	pub ws_xpixel: u16,
	/// The height of the window in pixels.
	pub ws_ypixel: u16,
}

/// Returns the width of a tab character for the given cursor X position
fn get_tab_size(cursor_x: usize) -> usize {
	TAB_SIZE - (cursor_x % TAB_SIZE)
}

/// Tells whether the `n`th line is in the range from `start` to `end` (excluded), wrapping around
/// the history buffer
fn is_in_range_wrapping(n: usize, start: usize, end: usize) -> bool {
	if start == end {
		false
	} else if start < end {
		(start..end).contains(&n)
	} else {
		(start..HISTORY_LINES).contains(&n) || (0..end).contains(&n)
	}
}

/// Returns the number of lines from the absolute Y position `start` to `end`, wrapping around the
/// history buffer
fn relative_y_distance(start: usize, end: usize) -> usize {
	if end >= start {
		end - start
	} else {
		(HISTORY_LINES - start) + end
	}
}

/// Rings the TTY's bell.
fn ring_bell() {
	// TODO
}

/// Sends a signal `sig` to the given process group `pgid`.
fn send_signal(sig: Signal, pgrp: Pid) {
	if pgrp == 0 {
		return;
	}
	if let Some(proc) = Process::get_by_pid(pgrp) {
		Process::kill_group(&proc, sig);
	}
}

/// A character on display on a TTY
#[derive(Clone, Copy)]
pub struct Char {
	/// The actual character
	c: char,
	/// Character's color information
	color: vga::Color,
}

impl Char {
	const fn empty() -> Char {
		Char {
			c: ' ',
			color: DEFAULT_COLOR,
		}
	}

	const fn to_vga(self) -> vga::Char {
		(self.c as u8 as vga::Char) | ((self.color as vga::Char) << 8)
	}
}

/// TTY display manager.
pub struct Display {
	/// The TTY's width, in characters
	width: usize,
	/// The TTY's height, in characters
	height: usize,

	/// The X position of the cursor in the history
	cursor_x: usize,
	/// The Y position of the cursor in the history
	cursor_y: usize,

	/// The Y position of the screen in the history
	screen_y: usize,
	/// The content of the TTY's history
	// TODO Vec stores capacity and length. We don't need those since we can determine them from
	// the size of the history and the width of the screen
	history: Vec<Char>,
	/// The framebuffer. If `None`, we use text mode
	framebuffer: Option<Framebuffer>,

	/// Top row of the scrolling region (DECSTBM), in screen-relative coordinates.
	scroll_top: usize,
	/// Bottom row of the scrolling region (DECSTBM), exclusive, in screen-relative coordinates.
	scroll_bottom: usize,

	/// The ANSI escape codes buffer.
	ansi_buffer: ANSIBuffer,

	/// Tells whether the cursor is currently visible on screen.
	cursor_visible: bool,
	/// The current color for the text to be written
	current_color: vga::Color,
}

impl Display {
	fn history_off(&self, x: usize, y: usize) -> usize {
		y * self.width + x
	}

	fn display_char(&self, c: Char, x: usize, y: usize) {
		// If the character isn't on screen, do nothing
		if !(self.screen_y..self.screen_y + self.height).contains(&y) {
			return;
		}
		let y = y - self.screen_y;
		const FONT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/font.hex"));
		if let Some(fb) = &self.framebuffer {
			let fb_ptr: *mut u8 = fb.addr().as_ptr();
			let bytes_per_pixel = fb.info().framebuffer_bpp.div_ceil(8) as usize;
			let pitch = fb.info().framebuffer_pitch as usize;
			// Draw char
			let code = c.c as usize;
			let data_off = code * 16;
			let data = &FONT[data_off..data_off + 16];
			let char_px_off = y * 16 * pitch + x * 8 * bytes_per_pixel;
			for char_y in 0..16 {
				for char_x in 0..8 {
					let index = char_y * 8 + char_x;
					let data_byte = index / 8;
					let data_bit = index % 8;
					let set = data[data_byte] & (0x80 >> data_bit) != 0;
					// TODO colors
					let mut val = 0;
					if set {
						val = !val;
					}
					let px_off = char_px_off + char_y * pitch + char_x * bytes_per_pixel;
					for i in 0..bytes_per_pixel {
						unsafe {
							fb_ptr.add(px_off + i).write_volatile(val);
						}
					}
				}
			}
		} else {
			let pos = y * self.width + x;
			unsafe {
				vga::text_buf().add(pos).write(c.to_vga());
			}
		}
	}

	fn update_display(&self) {
		for l in 0..self.height {
			let y = (self.screen_y + l) % HISTORY_LINES;
			for x in 0..self.width {
				self.display_char(self.history[self.history_off(x, y)], x, y);
			}
		}
	}

	fn scroll_display(&mut self, newlines: usize) {
		if let Some(fb) = &self.framebuffer {
			let fb_ptr: *mut u8 = fb.addr().as_ptr();
			let pitch = fb.info().framebuffer_pitch as usize;
			let scroll_bytes = newlines * 16 * pitch;
			let screen_bytes = self.height * 16 * pitch;
			unsafe {
				// Shift lines up
				ptr::copy(
					fb_ptr.add(scroll_bytes),
					fb_ptr,
					screen_bytes - scroll_bytes,
				);
				// Clear the newly exposed bottom lines
				ptr::write_bytes(fb_ptr.add(screen_bytes - scroll_bytes), 0, scroll_bytes);
			}
		} else {
			let ptr = vga::text_buf();
			let keep = (self.height - newlines) * self.width;
			unsafe {
				// Shift lines up
				ptr::copy(ptr.add(newlines * self.width), ptr, keep);
				// Clear the newly exposed bottom lines
				for i in keep..self.height * self.width {
					ptr.add(i).write(Char::empty().to_vga());
				}
			}
		}
	}

	fn clear_display(&self) {
		if let Some(fb) = &self.framebuffer {
			let fb_ptr: *mut u8 = fb.addr().as_ptr();
			unsafe {
				ptr::write_bytes(fb_ptr, 0, fb.len());
			}
		} else {
			let ptr = vga::text_buf();
			let len = self.width * self.height;
			for i in 0..len {
				unsafe {
					ptr.add(i).write(Char::empty().to_vga());
				}
			}
		}
	}

	/// Updates the TTY's cursor to the screen
	fn update_cursor(&self) {
		let y = relative_y_distance(self.screen_y, self.cursor_y);
		vga::move_cursor(self.cursor_x as _, y as _);
	}

	/// Hides or shows the cursor on screen.
	fn set_cursor_visible(&mut self, visible: bool) {
		self.cursor_visible = visible;
		if visible {
			vga::enable_cursor();
		} else {
			vga::disable_cursor();
		}
	}

	/// Reinitializes TTY's current attributes.
	fn reset_attrs(&mut self) {
		self.current_color = DEFAULT_COLOR;
	}

	/// Sets the current foreground color `color` for TTY.
	fn set_fgcolor(&mut self, color: vga::Color) {
		self.current_color &= !0x0f;
		self.current_color |= color & 0x0f;
	}

	/// Resets the current foreground color `color` for TTY.
	fn reset_fgcolor(&mut self) {
		self.set_fgcolor(DEFAULT_COLOR);
	}

	/// Sets the current background color `color` for TTY.
	fn set_bgcolor(&mut self, color: vga::Color) {
		self.current_color &= !0x70;
		self.current_color |= (color & 0x07) << 4;
	}

	/// Resets the current background color `color` for TTY.
	fn reset_bgcolor(&mut self) {
		self.set_bgcolor(DEFAULT_COLOR >> 4);
	}

	/// Swaps the foreground and background colors.
	fn swap_colors(&mut self) {
		let fg = self.current_color & 0x0f;
		let bg = (self.current_color >> 4) & 0x0f;
		self.set_fgcolor(bg);
		self.set_bgcolor(fg);
	}

	/// Sets the blinking state of the text for TTY.
	///
	/// If set to `true`, new text will blink. If set to `false`, new text will not blink.
	fn set_blinking(&mut self, blinking: bool) {
		if blinking {
			self.current_color |= 0x80;
		} else {
			self.current_color &= !0x80;
		}
	}

	/// Clears a range of the TTY's history.
	///
	/// Arguments:
	/// - `start_x` is the starting X coordinate of the history range to clear
	/// - `start_y` is the starting X coordinate of the history range to clear
	/// - `end_x` is the ending X coordinate of the history range to clear (excluded)
	/// - `end_y` is the ending Y coordinate of the history range to clear (included)
	fn clear_range(&mut self, start_x: usize, start_y: usize, end_x: usize, end_y: usize) {
		let start = self.history_off(start_x, start_y % HISTORY_LINES);
		let end = self.history_off(end_x, end_y % HISTORY_LINES);
		if start <= end {
			// Continuous in memory
			self.history[start..end].fill(Char::empty());
		} else {
			// Wrapping
			self.history[start..].fill(Char::empty());
			self.history[..end].fill(Char::empty());
		}
		// Update on screen
		self.update_display();
	}

	/// Clears all TTY's history.
	fn clear_all(&mut self) {
		self.cursor_x = 0;
		self.cursor_y = 0;
		self.screen_y = 0;
		self.history.fill(Char::empty());
		self.clear_display();
		self.update_cursor();
	}

	/// Inserts `n` blank lines at the cursor row, shifting rows below it down within the scrolling
	/// region. Rows shifted past the bottom of the region are discarded.
	fn insert_lines(&mut self, n: usize) {
		let screen_row = relative_y_distance(self.screen_y, self.cursor_y % HISTORY_LINES);
		if screen_row < self.scroll_top || screen_row >= self.scroll_bottom {
			return;
		}
		let max = self.scroll_bottom - screen_row;
		let n = min(n, max);
		let movable = max - n;
		for i in (0..movable).rev() {
			let src = (self.screen_y + screen_row + i) % HISTORY_LINES;
			let dst = (self.screen_y + screen_row + i + n) % HISTORY_LINES;
			self.history[dst] = self.history[src];
		}
		for i in 0..n {
			let row = (self.screen_y + screen_row + i) % HISTORY_LINES;
			self.history[row].fill(EMPTY_CHAR);
		}
		self.update_screen();
	}

	/// Deletes `n` rows starting at the cursor row, shifting subsequent rows up within the
	/// scrolling region. Blank rows are placed at the bottom of the region.
	fn delete_lines(&mut self, n: usize) {
		let screen_row = relative_y_distance(self.screen_y, self.cursor_y % HISTORY_LINES);
		if screen_row < self.scroll_top || screen_row >= self.scroll_bottom {
			return;
		}
		let max = self.scroll_bottom - screen_row;
		let n = min(n, max);
		let movable = max - n;
		for i in 0..movable {
			let src = (self.screen_y + screen_row + i + n) % HISTORY_LINES;
			let dst = (self.screen_y + screen_row + i) % HISTORY_LINES;
			self.history[dst] = self.history[src];
		}
		for i in 0..n {
			let row = (self.screen_y + screen_row + movable + i) % HISTORY_LINES;
			self.history[row].fill(EMPTY_CHAR);
		}
		self.update_screen();
	}

	/// Scrolls the contents of the scrolling region up by `n` rows. The top `n` rows of the region
	/// are discarded and the bottom `n` rows are blanked.
	fn scroll_region_up(&mut self, n: usize) {
		let top = self.scroll_top;
		let bottom = self.scroll_bottom;
		let size = bottom - top;
		let n = min(n, size);
		let movable = size - n;
		for i in 0..movable {
			let src = (self.screen_y + top + i + n) % HISTORY_LINES;
			let dst = (self.screen_y + top + i) % HISTORY_LINES;
			self.history[dst] = self.history[src];
		}
		for i in 0..n {
			let row = (self.screen_y + bottom - n + i) % HISTORY_LINES;
			self.history[row].fill(EMPTY_CHAR);
		}
		self.update_screen();
	}

	/// Sets the top and bottom margins of the scrolling region (DECSTBM).
	fn set_scroll_region(&mut self, top: usize, bottom: usize) {
		let height = vga::HEIGHT as usize;
		let top = min(top, height);
		let bottom = min(bottom, height);
		if top + 2 <= bottom {
			self.scroll_top = top;
			self.scroll_bottom = bottom;
		} else {
			self.scroll_top = 0;
			self.scroll_bottom = height;
		}
		// VT100 spec: DECSTBM homes the cursor.
		self.cursor_x = 0;
		self.cursor_y = self.screen_y;
	}

	/// If the cursor is out of the screen, append lines by shifting the screen relative to the
	/// history buffer, wrapping if the history buffer is exceeded.
	fn append_lines(&mut self) {
		let screen_y_end = (self.screen_y + self.height) % HISTORY_LINES;
		if is_in_range_wrapping(self.cursor_y, self.screen_y, screen_y_end) {
			return;
		}
		let newlines = relative_y_distance(screen_y_end, self.cursor_y % HISTORY_LINES) + 1;
		// Clear new lines
		let clear_start = self.history_off(0, screen_y_end);
		let new_screen_y_end = screen_y_end + newlines;
		if let Some(lines_after) = new_screen_y_end.checked_sub(HISTORY_LINES) {
			let clear_end = self.history_off(self.width, lines_after);
			self.history[clear_start..].fill(Char::empty());
			self.history[..clear_end].fill(Char::empty());
		} else {
			let clear_end = self.history_off(self.width, new_screen_y_end);
			self.history[clear_start..clear_end].fill(Char::empty());
		}
		// Update screen position
		self.screen_y = (self.screen_y + newlines) % HISTORY_LINES;
		self.cursor_y %= HISTORY_LINES;
		// Update display
		self.scroll_display(newlines);
	}

	/// Moves the cursor forward `n` characters.
	fn cursor_forward(&mut self, n: usize) {
		let off = self.cursor_x + n;
		self.cursor_x = off % self.width;
		let newlines = off / self.width;
		if newlines > 0 {
			self.cursor_y += newlines;
			self.append_lines();
		}
	}

	/// Moves the cursor backwards `n` characters.
	fn cursor_backward(&mut self, n: usize) {
		self.cursor_x = self.cursor_x.saturating_sub(n);
	}

	/// Moves the cursor `n` lines down.
	///
	/// If the cursor is at the bottom of a non-default scrolling region, scrolls the region up
	/// instead of advancing the cursor. Otherwise, advances the cursor and scrolls the screen
	/// through the history buffer as needed.
	fn newline(&mut self, n: usize) {
		self.cursor_x = 0;
		let region_partial = self.scroll_top != 0 || self.scroll_bottom != vga::HEIGHT as usize;
		for _ in 0..n {
			let screen_row = relative_y_distance(self.screen_y, self.cursor_y % HISTORY_LINES);
			if region_partial && screen_row + 1 == self.scroll_bottom {
				self.scroll_region_up(1);
			} else {
				self.cursor_y += 1;
				self.append_lines();
			}
		}
	}
}

/// TTY input manager.
struct Input {
	/// The buffer containing characters from TTY input.
	buf: [u8; INPUT_MAX],
	/// The current size of the input buffer.
	input_size: usize,
	/// The size of the data available to be read from the TTY.
	available_size: usize,
}

struct Settings {
	/// Terminal I/O settings.
	termios: Termios,
	/// The size of the TTY.
	winsize: WinSize,
	/// The current foreground Program Group ID.
	pgrp: Pid,
}

// TODO Use the values in winsize
/// A TTY.
pub struct TTY {
	/// Display manager.
	display: IntSpin<Display>,
	/// Input manager.
	input: IntSpin<Input>,
	/// TTY settings
	settings: IntSpin<Settings>,

	/// The queue of processes waiting for incoming data to read.
	rd_queue: WaitQueue,
}

/// The TTY.
pub static TTY: TTY = TTY {
	display: IntSpin::new(Display {
		width: vga::WIDTH as usize,
		height: vga::HEIGHT as usize,

		cursor_x: 0,
		cursor_y: 0,

		screen_y: 0,
		history: Vec::new(),
		framebuffer: None,

		scroll_top: 0,
		scroll_bottom: vga::HEIGHT as usize,

		ansi_buffer: ANSIBuffer::new(),

		cursor_visible: true,
		current_color: DEFAULT_COLOR,
	}),
	input: IntSpin::new(Input {
		buf: [0; INPUT_MAX],
		input_size: 0,
		available_size: 0,
	}),
	settings: IntSpin::new(Settings {
		pgrp: 0,
		termios: Termios::new(),
		winsize: WinSize {
			ws_row: vga::HEIGHT as _,
			ws_col: vga::WIDTH as _,
			ws_xpixel: vga::PIXEL_WIDTH as _,
			ws_ypixel: vga::PIXEL_HEIGHT as _,
		},
	}),

	rd_queue: WaitQueue::new(),
};

impl TTY {
	/// Shows the TTY on screen.
	///
	/// `fb` is the framebuffer. If `None`, text mode is used.
	pub fn show(&self, fb: Option<Framebuffer>) -> AllocResult<()> {
		let mut disp = self.display.lock();
		disp.framebuffer = fb;
		if let Some(fb) = &disp.framebuffer {
			let width = fb.info().framebuffer_width as usize / 8;
			let height = fb.info().framebuffer_height as usize / 16;
			disp.width = width;
			disp.height = height;
		}
		disp.history = vec![Char::empty(); disp.width * HISTORY_LINES]?;
		let cursor_visible = disp.cursor_visible;
		disp.set_cursor_visible(cursor_visible);
		disp.update_display();
		disp.update_cursor();
		Ok(())
	}

	/// Writes the character `c` to the TTY.
	fn putchar(&self, disp: &mut Display, mut c: char) {
		if self.get_termios().c_oflag & OLCUC != 0 {
			c = c.to_ascii_lowercase();
		}

		// TODO Implement ONLCR (Map NL to CR-NL)
		// TODO Implement ONOCR
		// TODO Implement ONLRET

		match c as u32 {
			0x07 => ring_bell(),
			// Tab (\t)
			0x09 => disp.cursor_forward(get_tab_size(disp.cursor_x)),
			// New Line (\n)
			0x0a => disp.newline(1),
			// Form Feed (^L)
			0x0c => {
				// TODO Move printer to a top of page
			}
			// Carriage Return (\r)
			0x0d => disp.cursor_x = 0,
			0x08 | 0x7f => disp.cursor_backward(1),
			// SO/SI: G0/G1 character-set switching. We only support a single charset, so ignore
			0x0e | 0x0f => {}
			_ => {
				let c = Char {
					c,
					color: disp.current_color,
				};
				let off = disp.history_off(disp.cursor_x, disp.cursor_y);
				disp.history[off] = c;
				disp.display_char(c, disp.cursor_x, disp.cursor_y);
				disp.cursor_forward(1);
			}
		}
	}

	/// Writes the content of `buf` to the TTY.
	pub fn write(&self, buf: &[u8]) {
		let mut display = self.display.lock();
		// If not init yet, do nothing
		if unlikely(display.history.is_empty()) {
			return;
		}
		let mut i = 0;
		while i < buf.len() {
			let c = buf[i];
			// Route through the ANSI handler when starting a new escape sequence OR continuing
			// one that was left partial by a previous `write()` call.
			if c == ESCAPE || !display.ansi_buffer.is_empty() {
				let j = ansi::handle(self, &mut display, &buf[i..]);
				if j > 0 {
					i += j;
					continue;
				}
			}
			// TODO handle unicode
			self.putchar(&mut display, c as char);
			i += 1;
		}
		display.update_cursor();
	}

	/// Injects bytes directly into the input buffer, bypassing ECHO and canonical mode
	/// processing.
	pub(crate) fn inject_input(&self, buffer: &[u8]) {
		{
			let mut input = self.input.lock();
			let off = input.input_size;
			let avail = input.buf.len() - off;
			let len = min(buffer.len(), avail);
			input.buf[off..off + len].copy_from_slice(&buffer[..len]);
			input.input_size += len;
			input.available_size += len;
		}
		self.rd_queue.wake_next();
	}

	// TODO Implement IUTF8
	/// Reads inputs from the TTY and writes it into the buffer `buf`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&self, buf: UserSlice<u8>) -> EResult<usize> {
		self.rd_queue.wait_until(|| {
			let mut input = self.input.lock();
			let termios = self.get_termios();
			// Canonical mode
			let canon = termios.c_lflag & ICANON != 0;
			let min_chars = if canon {
				1
			} else {
				termios.c_cc[VMIN] as usize
			};
			// If not enough data is available, wait
			if input.available_size < min_chars {
				return None;
			}
			let mut len = min(buf.len(), input.available_size);
			if canon {
				let eof = termios.c_cc[VEOF];
				let eof_off = input.buf[..len].iter().position(|v| *v == eof);
				if eof_off == Some(0) {
					// Shift data
					input.buf.rotate_left(1);
					input.input_size -= 1;
					input.available_size -= 1;
					return Some(Ok(0));
				}
				if let Some(eof_off) = eof_off {
					// Making the next call EOF
					len = eof_off;
				}
			} else {
				// Update available length
				len = min(buf.len(), input.available_size);
			}
			// Copy data
			let res = buf.copy_to_user(0, &input.buf[..len]);
			if let Err(e) = res {
				return Some(Err(e));
			}
			// Shift data
			input.buf.rotate_left(len);
			input.input_size -= len;
			input.available_size -= len;
			// Ring the bell if the buffer is full
			if termios.c_iflag & IMAXBEL != 0 && input.input_size >= buf.len() {
				ring_bell();
			}
			Some(Ok(len))
		})?
	}

	/// Tells whether the TTY has any data available to be read.
	pub fn has_input_available(&self) -> bool {
		let termios = self.get_termios();
		// Canonical mode
		let canon = termios.c_lflag & ICANON != 0;
		let min = if canon {
			1
		} else {
			termios.c_cc[VMIN] as usize
		};
		self.input.lock().available_size >= min
	}

	// TODO Implement IUTF8
	/// Takes the given string `buffer` as input, making it available from the
	/// terminal input.
	pub fn input(&self, buffer: &[u8]) {
		let termios = self.get_termios().clone();

		// Echo without holding the input lock to avoid display -> input vs input -> display lock
		// inversion against `write` (which holds `display` and may call `inject_input`)
		if termios.c_lflag & ECHO != 0 {
			self.write(buffer);
		}
		// TODO If ECHO is disabled but ICANON and ECHONL are set, print newlines

		// TODO Implement IGNBRK and BRKINT
		// TODO Implement parity checking
		// TODO IXON / IXANY / IXOFF

		// Mutate the input buffer under the input lock only.
		let mut erase_count = 0usize;
		let stored_len;
		{
			let mut input = self.input.lock();
			let len = min(buffer.len(), input.buf.len() - input.input_size);
			stored_len = len;
			let buffer = &buffer[..len];
			let input_size = input.input_size;
			utils::slice_copy(buffer, &mut input.buf[input_size..]);
			let new_bytes = &mut input.buf[input_size..(input_size + len)];
			for b in new_bytes {
				if termios.c_iflag & ISTRIP != 0 {
					*b &= 1 << 7;
				}
				// TODO Implement IGNCR (ignore carriage return)
				if termios.c_iflag & INLCR != 0 && *b == b'\n' {
					*b = b'\r';
				}
				if termios.c_iflag & ICRNL != 0 && *b == b'\r' {
					*b = b'\n';
				}
				if termios.c_iflag & IUCLC != 0 && (*b as char).is_ascii_uppercase() {
					*b = (*b as char).to_ascii_uppercase() as u8;
				}
			}
			input.input_size += len;
			if termios.c_lflag & ICANON != 0 {
				let mut i = input.input_size - len;
				while i < input.input_size {
					let b = input.buf[i];
					if b == termios.c_cc[VEOF] || b == b'\n' {
						input.available_size = i + 1;
						i += 1;
					} else if b == 0xf7 {
						// Drop the 0xf7 byte from the input buffer
						let end = input.input_size;
						input.buf.copy_within((i + 1)..end, i);
						input.input_size -= 1;
						erase_count += 1;
					} else {
						i += 1;
					}
				}
			} else {
				input.available_size = input.input_size;
			}
		}

		// Now-lock-free post-processing: visual erase, ECHOCTL printing, signal delivery.
		for _ in 0..erase_count {
			self.erase();
		}
		if termios.c_lflag & ISIG != 0 {
			let pgrp = self.get_pgrp();
			for b in &buffer[..stored_len] {
				if termios.c_lflag & (ECHO | ECHOCTL) == ECHO | ECHOCTL && *b >= 1 && *b < 32 {
					self.write(b"^A");
				}
				// TODO Handle every special characters
				if *b == termios.c_cc[VINTR] {
					send_signal(Signal::SIGINT, pgrp);
				} else if *b == termios.c_cc[VQUIT] {
					send_signal(Signal::SIGQUIT, pgrp);
				} else if *b == termios.c_cc[VSUSP] {
					send_signal(Signal::SIGTSTP, pgrp);
				}
			}
		}

		self.rd_queue.wake_next();
	}

	/// Erases `count` characters in TTY.
	pub fn erase(&self) {
		let termios = self.get_termios();
		// Release the input lock before taking the display lock so we don't form a cycle
		// against `write` -> `inject_input` (display -> input)
		let visual_erase;
		{
			let mut input = self.input.lock();
			if termios.c_lflag & ICANON != 0 {
				if input.input_size == 0 {
					return;
				}
				input.input_size -= 1;
				visual_erase = termios.c_lflag & ECHOE != 0;
			} else {
				drop(input);
				self.input(&[0x7f]);
				return;
			}
		}
		if visual_erase {
			let mut disp = self.display.lock();
			// TODO Handle tab characters
			disp.cursor_backward(1);
			let cursor_x = disp.cursor_x;
			let cursor_y = disp.cursor_y;
			let off = disp.history_off(cursor_x, cursor_y);
			disp.history[off] = Char::empty();
			disp.display_char(Char::empty(), cursor_x, cursor_y);
			disp.update_cursor();
		}
		self.rd_queue.wake_next();
	}

	/// Returns the current foreground Program Group ID.
	#[inline]
	pub fn get_pgrp(&self) -> Pid {
		self.settings.lock().pgrp
	}

	/// Sets the current foreground Program Group ID.
	#[inline]
	pub fn set_pgrp(&self, pgrp: Pid) {
		self.settings.lock().pgrp = pgrp;
	}

	/// Returns the terminal IO settings.
	pub fn get_termios(&self) -> Termios {
		self.settings.lock().termios.clone()
	}

	/// Sets the terminal IO settings.
	pub fn set_termios(&self, termios: Termios) {
		self.settings.lock().termios = termios;
	}

	/// Returns the window size of the TTY.
	pub fn get_winsize(&self) -> WinSize {
		self.settings.lock().winsize.clone()
	}

	/// Sets the window size of the TTY.
	///
	/// If a foreground process group is set on the TTY, the function shall send
	/// it a `SIGWINCH` signal.
	pub fn set_winsize(&self, mut winsize: WinSize) {
		// Clamping values
		if winsize.ws_col > vga::WIDTH as _ {
			winsize.ws_col = vga::WIDTH as _;
		}
		if winsize.ws_row > vga::HEIGHT as _ {
			winsize.ws_row = vga::HEIGHT as _;
		}
		// Changes to the size in pixels are ignored
		winsize.ws_xpixel = vga::PIXEL_WIDTH as _;
		winsize.ws_ypixel = vga::PIXEL_HEIGHT as _;

		self.settings.lock().winsize = winsize;
		send_signal(Signal::SIGWINCH, self.get_pgrp());
	}
}

/// Shows the initialization TTY on screen
pub(crate) fn show(boot_info: &BootInfo) {
	let mut warn = false;
	let fb = if let Some(fb_info) = boot_info.fb_info.clone() {
		let fb = Framebuffer::new(fb_info);
		warn = fb.is_none();
		fb
	} else {
		None
	};
	// Map VGA text buffer if using it
	if fb.is_none() {
		KERNEL_VMEM.map_range(
			vga::BUFFER_PHYS as _,
			vga::text_buf().into(),
			1,
			fb::MAP_FLAGS,
		);
	}
	TTY.show(fb)
		.expect("Could not allocate memory for TTY history");
	if warn {
		// TODO panic?
		println!("Warning: could not remap framebuffer, using text mode!");
	}
}
