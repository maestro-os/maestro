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
	file::wait_queue::WaitQueue,
	memory::{user::UserSlice, vmem},
	process::{Process, pid::Pid, signal::Signal},
	sync::spin::IntSpin,
	tty::{
		ansi::{ANSIBuffer, ESCAPE},
		termios::{Termios, consts::*},
	},
};
use core::{cmp::min, ptr};
use utils::errno::EResult;

/// The number of history lines for one TTY.
const HISTORY_LINES: vga::Pos = 128;
/// The number of characters a TTY can store.
const HISTORY_SIZE: usize = (vga::WIDTH as usize) * (HISTORY_LINES as usize);

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

/// Returns the position of the cursor in the history array from `x` and `y`
/// position.
fn get_history_offset(x: vga::Pos, y: vga::Pos) -> usize {
	let off = (y * vga::WIDTH + x) as usize;
	debug_assert!(off < HISTORY_SIZE);
	off
}

/// Returns the position of a tab character for the given cursor X position.
fn get_tab_size(cursor_x: vga::Pos) -> usize {
	TAB_SIZE - ((cursor_x as usize) % TAB_SIZE)
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
		proc.kill_group(sig);
	}
}

/// TTY display manager.
pub struct TTYDisplay {
	/// The X position of the cursor in the history
	cursor_x: vga::Pos,
	/// The Y position of the cursor in the history
	cursor_y: vga::Pos,

	/// The Y position of the screen in the history
	screen_y: vga::Pos,
	/// The content of the TTY's history
	history: [vga::Char; HISTORY_SIZE],

	/// Terminal I/O settings.
	termios: Termios,
	/// The size of the TTY.
	winsize: WinSize,
	/// The ANSI escape codes buffer.
	ansi_buffer: ANSIBuffer,

	/// The current foreground Program Group ID.
	pgrp: Pid,

	/// Tells whether the cursor is currently visible on screen.
	cursor_visible: bool,
	/// The current color for the text to be written
	current_color: vga::Color,
}

impl TTYDisplay {
	/// Updates the TTY to the screen.
	pub fn update(&mut self) {
		let buff = &self.history[get_history_offset(0, self.screen_y)];
		unsafe {
			vmem::write_ro(|| {
				ptr::copy_nonoverlapping(
					buff as *const vga::Char,
					vga::get_buffer_virt() as *mut vga::Char,
					(vga::WIDTH as usize) * (vga::HEIGHT as usize),
				);
			});
		}

		let y = self.cursor_y - self.screen_y;
		vga::move_cursor(self.cursor_x, y);
	}

	/// Shows the TTY on screen.
	pub fn show(&mut self) {
		self.set_cursor_visible(self.cursor_visible);
		self.update();
	}

	/// Hides or shows the cursor on screen.
	pub fn set_cursor_visible(&mut self, visible: bool) {
		self.cursor_visible = visible;
		if visible {
			vga::enable_cursor();
		} else {
			vga::disable_cursor();
		}
	}

	/// Reinitializes TTY's current attributes.
	pub fn reset_attrs(&mut self) {
		self.current_color = vga::DEFAULT_COLOR;
	}

	/// Sets the current foreground color `color` for TTY.
	pub fn set_fgcolor(&mut self, color: vga::Color) {
		self.current_color &= !0x7f;
		self.current_color |= color;
	}

	/// Resets the current foreground color `color` for TTY.
	pub fn reset_fgcolor(&mut self) {
		self.set_fgcolor(vga::DEFAULT_COLOR);
	}

	/// Sets the current background color `color` for TTY.
	pub fn set_bgcolor(&mut self, color: vga::Color) {
		self.current_color &= !((0x7f << 4) as vga::Color);
		self.current_color |= color << 4;
	}

	/// Resets the current background color `color` for TTY.
	pub fn reset_bgcolor(&mut self) {
		self.set_bgcolor(vga::DEFAULT_COLOR);
	}

	/// Swaps the foreground and background colors.
	pub fn swap_colors(&mut self) {
		let fg = self.current_color & 0x7f;
		let bg = self.current_color & (0x7f << 4);
		self.set_fgcolor(fg);
		self.set_bgcolor(bg);
	}

	/// Sets the blinking state of the text for TTY.
	///
	/// If set to `true`, new text will blink. If set to `false`, new text will not blink.
	pub fn set_blinking(&mut self, blinking: bool) {
		if blinking {
			self.current_color |= 0x80;
		} else {
			self.current_color &= !0x80;
		}
	}

	/// Clears the TTY's history.
	///
	/// Arguments:
	/// - `start`: is the start of the history range to clear
	/// - `end`: is the end of the history range to clear
	/// - `reset_cursor`: if set to `true`, the cursor will be reset to the top-left corner
	/// - `all`: if set to `true`, clear all the history
	pub fn clear(&mut self, start: usize, end: usize, reset_cursor: bool, all: bool) {
		if reset_cursor {
			self.cursor_x = 0;
			self.cursor_y = 0;
		}
		let slice = if all {
			// Clear all of history
			self.screen_y = 0;
			&mut self.history
		} else {
			// Clear only the given range
			&mut self.history[start..end]
		};
		slice.fill((vga::DEFAULT_COLOR as vga::Char) << 8);
		self.update();
	}

	/// Fixes the position of the cursor after executing an action.
	fn fix_pos(&mut self) {
		if self.cursor_x < 0 {
			let off = -self.cursor_x;
			self.cursor_x = vga::WIDTH - (off % vga::WIDTH);
			self.cursor_y -= off / vga::WIDTH + 1;
		}

		if self.cursor_x >= vga::WIDTH {
			let off = self.cursor_x;
			self.cursor_x = off % vga::WIDTH;
			self.cursor_y += off / vga::WIDTH;
		}

		if self.cursor_y < self.screen_y {
			self.screen_y = self.cursor_y;
		}

		if self.cursor_y >= self.screen_y + vga::HEIGHT {
			self.screen_y = self.cursor_y - vga::HEIGHT + 1;
		}

		if self.cursor_y >= HISTORY_LINES {
			self.cursor_y = HISTORY_LINES - 1;
		}

		if self.cursor_y < 0 {
			self.cursor_y = 0;
		}

		if self.screen_y < 0 {
			self.screen_y = 0;
		}

		if self.screen_y + vga::HEIGHT > HISTORY_LINES {
			let diff = ((self.screen_y + vga::HEIGHT - HISTORY_LINES) * vga::WIDTH) as usize;
			let size = self.history.len() - diff;
			for i in 0..size {
				self.history[i] = self.history[diff + i];
			}
			for i in size..self.history.len() {
				self.history[i] = (vga::DEFAULT_COLOR as vga::Char) << 8;
			}

			self.screen_y = HISTORY_LINES - vga::HEIGHT;
		}

		debug_assert!(self.cursor_x >= 0);
		debug_assert!(self.cursor_x < vga::WIDTH);
		debug_assert!(self.cursor_y - self.screen_y >= 0);
		debug_assert!(self.cursor_y - self.screen_y < vga::HEIGHT);
	}

	/// Moves the cursor forward `x` characters horizontally and `y` characters
	/// vertically.
	fn cursor_forward(&mut self, x: usize, y: usize) {
		self.cursor_x += x as vga::Pos;
		self.cursor_y += y as vga::Pos;
		self.fix_pos();
	}

	/// Moves the cursor backwards `x` characters horizontally and `y`
	/// characters vertically.
	fn cursor_backward(&mut self, x: usize, y: usize) {
		self.cursor_x -= x as vga::Pos;
		self.cursor_y -= y as vga::Pos;
		self.fix_pos();
	}

	/// Moves the cursor `n` lines down.
	fn newline(&mut self, n: usize) {
		self.cursor_x = 0;
		self.cursor_y += n as i16;
		self.fix_pos();
	}

	/// Writes the character `c` to the TTY.
	fn putchar(&mut self, mut c: u8) {
		if self.termios.c_oflag & OLCUC != 0 && (c as char).is_ascii_uppercase() {
			c = (c as char).to_ascii_lowercase() as u8;
		}

		// TODO Implement ONLCR (Map NL to CR-NL)
		// TODO Implement ONOCR
		// TODO Implement ONLRET

		match c {
			0x07 => ring_bell(),
			b'\t' => self.cursor_forward(get_tab_size(self.cursor_x), 0),
			b'\n' => self.newline(1),
			// Form Feed (^L)
			0x0c => {
				// TODO Move printer to a top of page
			}
			b'\r' => self.cursor_x = 0,
			0x08 | 0x7f => self.cursor_backward(1, 0),
			_ => {
				let tty_char = (c as vga::Char) | ((self.current_color as vga::Char) << 8);
				let pos = get_history_offset(self.cursor_x, self.cursor_y);
				self.history[pos] = tty_char;
				self.cursor_forward(1, 0);
			}
		}
	}

	/// Writes the content of `buf` to the TTY.
	pub fn write(&mut self, buf: &[u8]) {
		// TODO Add a compilation and/or runtime option for this
		serial::PORTS[0].lock().write(buf);

		let mut i = 0;
		while i < buf.len() {
			let c = buf[i];
			if c == ESCAPE {
				let j = ansi::handle(self, &buf[i..buf.len()]);
				if j > 0 {
					i += j;
					continue;
				}
			}
			self.putchar(c);
			i += 1;
		}
		self.update();
	}

	/// Returns the terminal IO settings.
	pub fn get_termios(&self) -> &Termios {
		&self.termios
	}

	/// Sets the terminal IO settings.
	pub fn set_termios(&mut self, termios: Termios) {
		self.termios = termios;
	}

	/// Returns the current foreground Program Group ID.
	pub fn get_pgrp(&self) -> Pid {
		self.pgrp
	}

	/// Sets the current foreground Program Group ID.
	pub fn set_pgrp(&mut self, pgrp: Pid) {
		self.pgrp = pgrp;
	}

	/// Returns the window size of the TTY.
	pub fn get_winsize(&self) -> &WinSize {
		&self.winsize
	}

	/// Sets the window size of the TTY.
	///
	/// If a foreground process group is set on the TTY, the function shall send
	/// it a `SIGWINCH` signal.
	pub fn set_winsize(&mut self, mut winsize: WinSize) {
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

		self.winsize = winsize;

		// Send a SIGWINCH if a process group is present
		send_signal(Signal::SIGWINCH, self.pgrp); // FIXME: deadlock when using `strace`
	}
}

/// TTY input manager.
struct TTYInput {
	/// The buffer containing characters from TTY input.
	buf: [u8; INPUT_MAX],
	/// The current size of the input buffer.
	input_size: usize,
	/// The size of the data available to be read from the TTY.
	available_size: usize,
}

// TODO Use the values in winsize
/// A TTY.
pub struct TTY {
	/// Display manager.
	pub display: IntSpin<TTYDisplay>,
	/// Input manager.
	input: IntSpin<TTYInput>,
	/// The queue of processes waiting for incoming data to read.
	rd_queue: WaitQueue,
}

/// The TTY.
pub static TTY: TTY = TTY {
	display: IntSpin::new(TTYDisplay {
		cursor_x: 0,
		cursor_y: 0,

		screen_y: 0,
		history: [(vga::DEFAULT_COLOR as vga::Char) << 8; HISTORY_SIZE],

		termios: Termios::new(),
		winsize: WinSize {
			ws_row: vga::HEIGHT as _,
			ws_col: vga::WIDTH as _,
			ws_xpixel: vga::PIXEL_WIDTH as _,
			ws_ypixel: vga::PIXEL_HEIGHT as _,
		},
		ansi_buffer: ANSIBuffer::new(),

		pgrp: 0,

		cursor_visible: true,
		current_color: vga::DEFAULT_COLOR,
	}),
	input: IntSpin::new(TTYInput {
		buf: [0; INPUT_MAX],
		input_size: 0,
		available_size: 0,
	}),
	rd_queue: WaitQueue::new(),
};

impl TTY {
	// TODO Implement IUTF8
	/// Reads inputs from the TTY and writes it into the buffer `buf`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&self, buf: UserSlice<u8>) -> EResult<usize> {
		self.rd_queue.wait_until(|| {
			let termios = self.display.lock().get_termios().clone();
			let mut input = self.input.lock();
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
		let display = self.display.lock();
		let input = self.input.lock();
		// Canonical mode
		let canon = display.termios.c_lflag & ICANON != 0;
		let min = if canon {
			1
		} else {
			display.get_termios().c_cc[VMIN] as usize
		};
		input.available_size >= min
	}

	// TODO Implement IUTF8
	/// Takes the given string `buffer` as input, making it available from the
	/// terminal input.
	pub fn input(&self, buffer: &[u8]) {
		let termios = self.display.lock().get_termios().clone();
		let mut input = self.input.lock();
		// The length to write to the input buffer
		let len = min(buffer.len(), input.buf.len() - input.input_size);
		// The slice containing the input
		let buffer = &buffer[..len];

		if termios.c_lflag & ECHO != 0 {
			// Write onto the TTY
			self.display.lock().write(buffer);
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
					// TODO Check
					self.erase(1);
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
				if termios.c_lflag & ECHO != 0
					&& termios.c_lflag & ECHOCTL != 0
					&& *b >= 1 && *b < 32
				{
					self.display.lock().write(&[b'^', b + b'A']);
				}

				// TODO Handle every special characters
				let pgrp = self.display.lock().pgrp;
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
	pub fn erase(&self, count: usize) {
		let termios = self.display.lock().termios.clone();
		let mut input = self.input.lock();
		if termios.c_lflag & ICANON != 0 {
			let count = min(count, input.buf.len());
			if count > input.input_size {
				return;
			}

			if termios.c_lflag & ECHOE != 0 {
				let mut disp = self.display.lock();
				// TODO Handle tab characters
				disp.cursor_backward(count, 0);
				let begin = get_history_offset(disp.cursor_x, disp.cursor_y);
				disp.history[begin..(begin + count)].fill(EMPTY_CHAR);
				disp.update();
			}

			input.input_size -= count;
		} else {
			// Printing DEL characters
			for _ in 0..count {
				self.input(&[0x7f]);
			}
		}

		self.rd_queue.wake_next();
	}
}
