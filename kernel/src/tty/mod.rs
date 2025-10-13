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
	device::serial,
	memory::{user::UserSlice, vmem},
	process::{Process, pid::Pid, signal::Signal},
	sync::{spin::IntSpin, wait_queue::WaitQueue},
	tty::{
		ansi::{ANSIBuffer, ESCAPE},
		termios::{Termios, consts::*},
	},
};
use core::{cmp::min, ptr};
use utils::errno::EResult;

/// The number of history lines for one TTY.
const HISTORY_LINES: usize = 128;

/// An empty character.
const EMPTY_CHAR: vga::Char = (vga::DEFAULT_COLOR as vga::Char) << 8;

/// The size of a tabulation in space-equivalent.
const TAB_SIZE: usize = 4;

/// The maximum number of characters in the input buffer of a TTY.
const INPUT_MAX: usize = 4096;

// TODO Implement character size mask
// TODO Full implement serial

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

/// TTY display manager.
pub struct Display {
	/// The X position of the cursor in the history
	cursor_x: usize,
	/// The Y position of the cursor in the history
	cursor_y: usize,

	/// The Y position of the screen in the history
	screen_y: usize,
	/// The content of the TTY's history
	history: [[vga::Char; vga::WIDTH as usize]; HISTORY_LINES],

	/// The ANSI escape codes buffer.
	ansi_buffer: ANSIBuffer,

	/// Tells whether the cursor is currently visible on screen.
	cursor_visible: bool,
	/// The current color for the text to be written
	current_color: vga::Color,
}

impl Display {
	/// Updates the TTY's text to the screen.
	fn update_screen(&self) {
		unsafe {
			vmem::write_ro(|| {
				let screen_end_y = self.screen_y + vga::HEIGHT as usize;
				if let Some(lines_after) = screen_end_y.checked_sub(HISTORY_LINES) {
					// Wraps around the TTY's history, we need two copies
					let lines_before = vga::HEIGHT as usize - lines_after;
					ptr::copy_nonoverlapping(
						&self.history[self.screen_y][0],
						vga::get_buffer_virt(),
						vga::WIDTH as usize * lines_before,
					);
					// Second copy
					ptr::copy_nonoverlapping(
						&self.history[0][0],
						vga::get_buffer_virt().add(vga::WIDTH as usize * lines_before),
						vga::WIDTH as usize * lines_after,
					);
				} else {
					// We can copy everything at once
					ptr::copy_nonoverlapping(
						&self.history[self.screen_y][0],
						vga::get_buffer_virt(),
						vga::WIDTH as usize * vga::HEIGHT as usize,
					);
				}
			});
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
		self.current_color = vga::DEFAULT_COLOR;
	}

	/// Sets the current foreground color `color` for TTY.
	fn set_fgcolor(&mut self, color: vga::Color) {
		self.current_color &= !0x7f;
		self.current_color |= color;
	}

	/// Resets the current foreground color `color` for TTY.
	fn reset_fgcolor(&mut self) {
		self.set_fgcolor(vga::DEFAULT_COLOR);
	}

	/// Sets the current background color `color` for TTY.
	fn set_bgcolor(&mut self, color: vga::Color) {
		self.current_color &= !((0x7f << 4) as vga::Color);
		self.current_color |= color << 4;
	}

	/// Resets the current background color `color` for TTY.
	fn reset_bgcolor(&mut self) {
		self.set_bgcolor(vga::DEFAULT_COLOR);
	}

	/// Swaps the foreground and background colors.
	fn swap_colors(&mut self) {
		let fg = self.current_color & 0x7f;
		let bg = self.current_color & (0x7f << 4);
		self.set_fgcolor(fg);
		self.set_bgcolor(bg);
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
		let start_y = start_y % HISTORY_LINES;
		let end_y = end_y % HISTORY_LINES;
		if start_y == end_y {
			self.history[start_y][start_x..end_x].fill(EMPTY_CHAR);
		} else if start_y < end_y {
			// Continuous in memory
			self.history[start_y..end_y].as_flattened_mut()[start_x..].fill(EMPTY_CHAR);
			self.history[end_y][..end_x].fill(EMPTY_CHAR);
		} else {
			// Wrapping
			self.history[start_y..].as_flattened_mut()[start_x..].fill(EMPTY_CHAR);
			self.history[..end_y].as_flattened_mut().fill(EMPTY_CHAR);
			self.history[end_y][..end_x].fill(EMPTY_CHAR);
		}
		self.update_screen();
	}

	/// Clears all TTY's history.
	fn clear_all(&mut self) {
		self.cursor_x = 0;
		self.cursor_y = 0;
		self.screen_y = 0;
		self.history.as_flattened_mut().fill(EMPTY_CHAR);
		self.update_screen();
		self.update_cursor();
	}

	/// If the cursor is out of the screen, append lines by shifting the screen relative to the
	/// history buffer, wrapping if the history buffer is exceeded.
	fn append_lines(&mut self) {
		let screen_y_end = (self.screen_y + vga::HEIGHT as usize) % HISTORY_LINES;
		if is_in_range_wrapping(self.cursor_y, self.screen_y, screen_y_end) {
			return;
		}
		let newlines = relative_y_distance(screen_y_end, self.cursor_y % HISTORY_LINES) + 1;
		// Clear new lines
		let new_screen_y_end = screen_y_end + newlines;
		if let Some(lines_after) = new_screen_y_end.checked_sub(HISTORY_LINES) {
			self.history[screen_y_end..]
				.as_flattened_mut()
				.fill(EMPTY_CHAR);
			self.history[..lines_after]
				.as_flattened_mut()
				.fill(EMPTY_CHAR);
		} else {
			self.history[screen_y_end..new_screen_y_end]
				.as_flattened_mut()
				.fill(EMPTY_CHAR);
		}
		// Update screen position
		self.screen_y = (self.screen_y + newlines) % HISTORY_LINES;
		self.cursor_y %= HISTORY_LINES;
	}

	/// Moves the cursor forward `n` characters.
	fn cursor_forward(&mut self, n: usize) {
		let off = self.cursor_x + n;
		self.cursor_x = off % vga::WIDTH as usize;
		let newlines = off / vga::WIDTH as usize;
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
	fn newline(&mut self, n: usize) {
		self.cursor_x = 0;
		self.cursor_y += n;
		self.append_lines();
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
		cursor_x: 0,
		cursor_y: 0,

		screen_y: 0,
		history: [[EMPTY_CHAR; vga::WIDTH as usize]; HISTORY_LINES],

		ansi_buffer: ANSIBuffer::new(),

		cursor_visible: true,
		current_color: vga::DEFAULT_COLOR,
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
	pub fn show(&self) {
		let mut disp = self.display.lock();
		let cursor_visible = disp.cursor_visible;
		disp.set_cursor_visible(cursor_visible);
		disp.update_screen();
		disp.update_cursor();
	}

	/// Writes the character `c` to the TTY.
	fn putchar(&self, disp: &mut Display, mut c: u8) {
		if self.get_termios().c_oflag & OLCUC != 0 && (c as char).is_ascii_uppercase() {
			c = (c as char).to_ascii_lowercase() as u8;
		}

		// TODO Implement ONLCR (Map NL to CR-NL)
		// TODO Implement ONOCR
		// TODO Implement ONLRET

		match c {
			0x07 => ring_bell(),
			b'\t' => disp.cursor_forward(get_tab_size(disp.cursor_x)),
			b'\n' => disp.newline(1),
			// Form Feed (^L)
			0x0c => {
				// TODO Move printer to a top of page
			}
			b'\r' => disp.cursor_x = 0,
			0x08 | 0x7f => disp.cursor_backward(1),
			_ => {
				let tty_char = (c as vga::Char) | ((disp.current_color as vga::Char) << 8);
				disp.history[disp.cursor_y][disp.cursor_x] = tty_char;
				disp.cursor_forward(1);
			}
		}
	}

	/// Writes the content of `buf` to the TTY.
	pub fn write(&self, buf: &[u8]) {
		// TODO Add a compilation and/or runtime option for this
		serial::PORTS[0].lock().write(buf);
		let mut display = self.display.lock();
		let mut i = 0;
		while i < buf.len() {
			let c = buf[i];
			if c == ESCAPE {
				let j = ansi::handle(self, &mut display, &buf[i..buf.len()]);
				if j > 0 {
					i += j;
					continue;
				}
			}
			self.putchar(&mut display, c);
			i += 1;
		}
		display.update_screen();
		display.update_cursor();
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
		let mut input = self.input.lock();
		// The length to write to the input buffer
		let len = min(buffer.len(), input.buf.len() - input.input_size);
		// The slice containing the input
		let buffer = &buffer[..len];

		if termios.c_lflag & ECHO != 0 {
			// Write onto the TTY
			self.write(buffer);
		}
		// TODO If ECHO is disabled but ICANON and ECHONL are set, print newlines

		// TODO Implement IGNBRK and BRKINT
		// TODO Implement parity checking

		// Writing to the input buffer
		// TODO Put in a different function
		{
			let input_size = input.input_size;
			utils::slice_copy(buffer, &mut input.buf[input_size..]);
			let new_bytes = &mut input.buf[input_size..(input_size + len)];

			for b in new_bytes {
				if termios.c_iflag & ISTRIP != 0 {
					// Stripping eighth bit
					*b &= 1 << 7;
				}

				// TODO Implement IGNCR (ignore carriage return)

				if termios.c_iflag & INLCR != 0 {
					// Translating NL to CR
					if *b == b'\n' {
						*b = b'\r';
					}
				}

				if termios.c_iflag & ICRNL != 0 {
					// Translating CR to NL
					if *b == b'\r' {
						*b = b'\n';
					}
				}

				if termios.c_iflag & IUCLC != 0 {
					// Translating uppercase characters to lowercase
					if (*b as char).is_ascii_uppercase() {
						*b = (*b as char).to_ascii_uppercase() as u8;
					}
				}
			}
			input.input_size += len;
		}

		// TODO IXON
		// TODO IXANY
		// TODO IXOFF

		if termios.c_lflag & ICANON != 0 {
			// Processing input
			let mut i = input.input_size - len;
			while i < input.input_size {
				let b = input.buf[i];

				if b == termios.c_cc[VEOF] || b == b'\n' {
					// Making the input available for reading
					input.available_size = i + 1;

					i += 1;
				} else if b == 0xf7 {
					self.erase();
				} else {
					i += 1;
				}
			}
		} else {
			// Making the input available for reading
			input.available_size = input.input_size;
		}

		// Sending signals if enabled
		if termios.c_lflag & ISIG != 0 {
			for b in buffer {
				// Printing special control characters if enabled
				if termios.c_lflag & (ECHO | ECHOCTL) == ECHO | ECHOCTL && *b >= 1 && *b < 32 {
					self.write(b"^A");
				}

				// TODO Handle every special characters
				let pgrp = self.get_pgrp();
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
		let mut input = self.input.lock();
		if termios.c_lflag & ICANON != 0 {
			if input.input_size == 0 {
				return;
			}
			if termios.c_lflag & ECHOE != 0 {
				let mut disp = self.display.lock();
				// TODO Handle tab characters
				disp.cursor_backward(1);
				let cursor_x = disp.cursor_x;
				let cursor_y = disp.cursor_y;
				disp.history[cursor_y][cursor_x] = EMPTY_CHAR;
				disp.update_screen();
				disp.update_cursor();
			}
			input.input_size -= 1;
		} else {
			self.input(&[0x7f]);
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
