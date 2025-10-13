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

//! ANSI escape sequences allow to control the terminal by specifying commands in standard output
//! of the terminal.

use super::{Display, TTY};
use crate::tty::{
	vga,
	vga::{Color, HEIGHT, WIDTH},
};
use core::{cmp::min, str};

/// The character used to initialize ANSI escape sequences.
pub const ESCAPE: u8 = 0x1b;
/// The Control Sequence Introducer character.
const CSI: u8 = b'[';

/// The size of the buffer used to parse ANSI escape codes.
pub const BUFFER_SIZE: usize = 128;
/// The maximum number of elements in a sequence.
pub const SEQ_MAX: usize = 5;

/// Enumeration of possible states of the ANSI parser.
pub(super) enum ANSIState {
	/// The sequence is valid has been executed and has been pruned from the buffer.
	Valid,
	/// The sequence is incomplete. Waiting for more data.
	Incomplete,
	/// The sequence is invalid, it has been printed as normal characters and has been pruned from
	/// the buffer.
	Invalid,
}

/// A FIFO buffer for ANSI escape sequences.
pub(super) struct ANSIBuffer {
	/// The buffer.
	buf: [u8; BUFFER_SIZE],
	/// The position of the cursor in the buffer.
	cursor: usize,
}

impl ANSIBuffer {
	/// Creates a new instance.
	pub const fn new() -> Self {
		Self {
			buf: [0; BUFFER_SIZE],
			cursor: 0,
		}
	}

	/// Returns the number of bytes in the buffer.
	pub fn len(&self) -> usize {
		self.cursor
	}

	/// Tells whether the buffer is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Pushes the data from the given buffer `buffer` into the current buffer.
	///
	/// If more characters are pushed than the remaining capacity, the function
	/// truncates the data to be pushed.
	///
	/// The function returns the number of characters that have been pushed.
	pub fn push_back(&mut self, buffer: &[u8]) -> usize {
		let len = min(buffer.len(), BUFFER_SIZE - self.cursor);
		self.buf[self.cursor..(self.cursor + len)].copy_from_slice(&buffer[..len]);
		self.cursor += len;
		len
	}

	/// Removes the first `n` characters from the buffer.
	pub fn pop_front(&mut self, n: usize) {
		self.buf.rotate_left(n);
		self.cursor -= n;
	}

	/// Clears the buffer.
	pub fn clear(&mut self) {
		self.cursor = 0;
	}
}

/// A view on an [`ANSIBuffer`], used to parse sequences.
///
/// Consuming data on the view doesn't affect the underlying buffer. Only the view itself.
struct ANSIBufferView<'tty> {
	/// The TTY.
	tty: &'tty mut Display,
	/// The current offset of the view in the buffer.
	cursor: usize,
}

impl<'tty> ANSIBufferView<'tty> {
	/// Creates a view the buffer of the given TTY.
	fn new(tty: &'tty mut Display) -> Self {
		Self {
			tty,
			cursor: 0,
		}
	}

	/// Returns an immutable reference to the underlying buffer view.
	pub fn buffer(&self) -> &[u8] {
		&self.tty.ansi_buffer.buf[..self.tty.ansi_buffer.cursor]
	}

	/// Returns the number of consumed characters.
	fn consumed_count(&self) -> usize {
		self.cursor
	}

	/// Peeks the next single character.
	///
	/// If not enough data remains, the function returns `None`.
	fn peek_char(&mut self) -> Option<u8> {
		self.buffer().get(self.cursor).cloned()
	}

	/// Consumes the next single character.
	///
	/// If not enough data remains, the function returns `None`.
	fn next_char(&mut self) -> Option<u8> {
		let c = self.peek_char()?;
		self.cursor += 1;
		Some(c)
	}

	/// Consumes the next number.
	///
	/// A successful return does not necessarily mean the number is complete. The buffer might be
	/// incomplete and need more data.
	///
	/// If not enough data remains or if the number is invalid, the function returns `None`.
	fn next_nbr(&mut self) -> Option<usize> {
		let nbr_len = utils::nbr_len(&self.buffer()[self.cursor..]);
		let Ok(nbr) = str::from_utf8(&self.buffer()[self.cursor..(self.cursor + nbr_len)]) else {
			return None;
		};
		let n = str::parse::<usize>(nbr).ok()?;
		self.cursor += nbr_len;
		Some(n)
	}

	/// Consumes the next sequence of `;`-separated numbers.
	///
	/// The function takes a buffer to write the sequence on. If the buffer is not large enough to
	/// fit the whole sequence, it is truncated.
	fn next_nbr_sequence<'b>(&mut self, buf: &'b mut [usize]) -> &'b [usize] {
		let mut len = 0;
		for b in buf.iter_mut() {
			let Some(nbr) = self.next_nbr() else {
				break;
			};
			*b = nbr;
			len += 1;
			// skip `;`
			if self.peek_char() != Some(b';') {
				break;
			}
			self.cursor += 1;
		}
		// skip remaining numbers of the sequence
		loop {
			if self.next_nbr().is_some() {
				continue;
			}
			if self.peek_char() == Some(b';') {
				self.cursor += 1;
				continue;
			}
			break;
		}
		&buf[..len]
	}
}

/// Returns the VGA color associated with the given command.
fn get_vga_color_from_cmd(cmd: u8) -> Color {
	match cmd {
		30 | 40 => vga::COLOR_BLACK,
		31 | 41 => vga::COLOR_RED,
		32 | 42 => vga::COLOR_GREEN,
		33 | 43 => vga::COLOR_BROWN,
		34 | 44 => vga::COLOR_BLUE,
		35 | 45 => vga::COLOR_MAGENTA,
		36 | 46 => vga::COLOR_CYAN,
		37 | 47 => vga::COLOR_LIGHT_GREY,
		90 | 100 => vga::COLOR_DARK_GREY,
		91 | 101 => vga::COLOR_LIGHT_RED,
		92 | 102 => vga::COLOR_LIGHT_GREEN,
		93 | 103 => vga::COLOR_YELLOW,
		94 | 104 => vga::COLOR_LIGHT_BLUE,
		95 | 105 => vga::COLOR_LIGHT_MAGENTA,
		96 | 106 => vga::COLOR_LIGHT_CYAN,
		97 | 107 => vga::COLOR_WHITE,
		_ => vga::COLOR_BLACK,
	}
}

/// Returns the VGA color associated with the given ID.
fn get_vga_color_from_id(id: u8) -> Color {
	match id {
		0 => vga::COLOR_BLACK,
		1 => vga::COLOR_RED,
		2 => vga::COLOR_GREEN,
		3 => vga::COLOR_BROWN,
		4 => vga::COLOR_BLUE,
		5 => vga::COLOR_MAGENTA,
		6 => vga::COLOR_CYAN,
		7 => vga::COLOR_LIGHT_GREY,
		8 => vga::COLOR_DARK_GREY,
		9 => vga::COLOR_LIGHT_RED,
		10 => vga::COLOR_LIGHT_GREEN,
		11 => vga::COLOR_YELLOW,
		12 => vga::COLOR_LIGHT_BLUE,
		13 => vga::COLOR_LIGHT_MAGENTA,
		14 => vga::COLOR_LIGHT_CYAN,
		15 => vga::COLOR_WHITE,
		_ => vga::COLOR_BLACK,
	}
}

/// Moves the cursor on TTY `tty`.
///
/// Arguments:
/// - `d` is the direction character.
/// - `n` is the number of cells to travel. If `None`, the default is used (`1`).
fn move_cursor(tty: &mut Display, d: u8, n: usize) -> ANSIState {
	let n = n.clamp(0, i16::MAX as usize);
	match d {
		b'A' => tty.cursor_y = tty.cursor_y.saturating_sub(n),
		b'B' => {
			let n = tty.cursor_y.checked_add(n).unwrap_or(HEIGHT as usize - 1);
			tty.cursor_y = min(n, HEIGHT as usize - 1);
		}
		b'C' => {
			let n = tty.cursor_x.checked_add(n).unwrap_or(WIDTH as usize - 1);
			tty.cursor_x = min(n, WIDTH as usize - 1);
		}
		b'D' => tty.cursor_x = tty.cursor_x.saturating_sub(n),
		_ => return ANSIState::Invalid,
	}
	ANSIState::Valid
}

/// Handles a Select Graphics Renderition (SGR) command.
///
/// `seq` is the id of the numbers describing the command.
fn parse_sgr(tty: &mut Display, seq: &[usize]) -> ANSIState {
	if seq.is_empty() {
		tty.reset_attrs();
		return ANSIState::Valid;
	}
	let mut iter = seq.iter();
	while let Some(cmd) = iter.next() {
		match *cmd {
			0 => tty.reset_attrs(),
			1 => {} // TODO Bold
			2 => {} // TODO Faint
			3 => {} // TODO Italic
			4 => {} // TODO Underline
			5 | 6 => tty.set_blinking(true),
			7 => tty.swap_colors(),
			8 => {}
			9 => {}  // TODO Crossed-out
			10 => {} // TODO Primary font
			11 => {} // TODO Alternative font
			12 => {} // TODO Alternative font
			13 => {} // TODO Alternative font
			14 => {} // TODO Alternative font
			15 => {} // TODO Alternative font
			16 => {} // TODO Alternative font
			17 => {} // TODO Alternative font
			18 => {} // TODO Alternative font
			19 => {} // TODO Alternative font
			20 | 21 => {}
			22 => {} // TODO Normal intensity
			23 => {} // TODO Not italic
			24 => {} // TODO Not underlined
			25 => tty.set_blinking(false),
			26 => {}
			27 => {} // TODO Not reversed
			28 => {}
			29 => {} // TODO Not crossed-out
			c @ (30..=37 | 90..=97) => {
				tty.set_fgcolor(get_vga_color_from_cmd(c as _));
			}
			38 => match iter.next() {
				Some(2) => {
					// TODO with VGA, use closest color
				}
				Some(5) => {
					let Some(nbr) = iter.next() else {
						return ANSIState::Invalid;
					};
					tty.set_fgcolor(get_vga_color_from_id(*nbr as _));
				}
				_ => return ANSIState::Invalid,
			},
			39 => tty.reset_fgcolor(),
			c @ (40..=47 | 100..=107) => {
				tty.set_bgcolor(get_vga_color_from_cmd(c as _));
			}
			48 => match iter.next() {
				Some(2) => {
					// TODO with VGA, use closest color
				}
				Some(5) => {
					let Some(nbr) = iter.next() else {
						return ANSIState::Invalid;
					};
					tty.set_bgcolor(get_vga_color_from_id(*nbr as _));
				}
				_ => return ANSIState::Invalid,
			},
			49 => tty.reset_bgcolor(),
			50..=107 => {}
			_ => return ANSIState::Invalid,
		}
	}
	ANSIState::Valid
}

/// Parses the CSI sequence in the given buffer view.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse_csi(view: &mut ANSIBufferView) -> ANSIState {
	let mut seq_buf: [usize; SEQ_MAX] = [0; SEQ_MAX];
	let seq = view.next_nbr_sequence(&mut seq_buf);
	let Some(cmd) = view.next_char() else {
		return ANSIState::Incomplete;
	};
	match (seq, cmd) {
		(_, b'?') => match (view.next_nbr(), view.next_char()) {
			(Some(7 | 25), Some(b'h')) => view.tty.set_cursor_visible(true),
			(Some(7 | 25), Some(b'l')) => view.tty.set_cursor_visible(false),
			_ => return ANSIState::Invalid,
		},
		(seq, b'A'..=b'D') => {
			return move_cursor(view.tty, cmd, seq.first().cloned().unwrap_or(1));
		}
		(seq, b'E') => view.tty.newline(seq.first().cloned().unwrap_or(1)),
		(_seq, b'F') => {
			// TODO Previous line
		}
		(seq, b'G') => {
			let y = seq
				.first()
				.cloned()
				.unwrap_or(1)
				.clamp(1, WIDTH as usize + 1)
				- 1;
			view.tty.cursor_y = y;
		}
		(seq, b'H') => {
			let x = seq
				.first()
				.cloned()
				.unwrap_or(1)
				.clamp(1, WIDTH as usize + 1)
				- 1;
			let y = seq
				.get(1)
				.cloned()
				.unwrap_or(1)
				.clamp(1, HEIGHT as usize + 1)
				- 1;
			view.tty.cursor_x = x;
			view.tty.cursor_y = view.tty.screen_y + y;
		}
		(seq, b'J') => {
			// Erase in display
			match seq.first().cloned().unwrap_or(0) {
				0 => view.tty.clear_range(
					view.tty.cursor_x,
					view.tty.cursor_y,
					WIDTH as usize,
					view.tty.screen_y + HEIGHT as usize - 1,
				),
				1 => view.tty.clear_range(
					0,
					view.tty.screen_y,
					view.tty.cursor_x,
					view.tty.cursor_y,
				),
				2 => view.tty.clear_range(
					0,
					view.tty.screen_y,
					WIDTH as usize,
					view.tty.screen_y + HEIGHT as usize - 1,
				),
				3 => view.tty.clear_all(),
				_ => return ANSIState::Invalid,
			}
		}
		(seq, b'K') => {
			// Erase in line
			match seq.first().cloned().unwrap_or(0) {
				0 => view.tty.clear_range(
					view.tty.cursor_x,
					view.tty.cursor_y,
					WIDTH as usize,
					view.tty.cursor_y,
				),
				1 => view.tty.clear_range(
					0,
					view.tty.cursor_y,
					view.tty.cursor_x,
					view.tty.cursor_y,
				),
				2 => view
					.tty
					.clear_range(0, view.tty.cursor_y, WIDTH as usize, view.tty.cursor_y),
				_ => return ANSIState::Invalid,
			}
		}
		(_seq, b'S') => {
			// TODO Scroll up
		}
		(_seq, b'T') => {
			// TODO Scroll down
		}
		(seq, b'm') => return parse_sgr(view.tty, seq),
		_ => return ANSIState::Invalid,
	}
	view.tty.update_screen();
	ANSIState::Valid
}

/// Parses the sequence in the given buffer.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse(view: &mut ANSIBufferView) -> ANSIState {
	if view.next_char() != Some(ESCAPE) {
		return ANSIState::Invalid;
	}
	match view.next_char() {
		Some(CSI) => parse_csi(view),
		// TODO
		None => ANSIState::Incomplete,
		_ => ANSIState::Invalid,
	}
}

/// Handles an ANSI escape sequences stored into the buffer `buffer` on the TTY `tty`.
///
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is
/// undefined.
///
/// The function returns the number of bytes consumed by the function.
pub(super) fn handle(tty: &TTY, disp: &mut Display, buffer: &[u8]) -> usize {
	disp.ansi_buffer.push_back(buffer);
	let mut n = 0;
	while !disp.ansi_buffer.is_empty() {
		let mut view = ANSIBufferView::new(disp);
		if view.peek_char() != Some(ESCAPE) {
			disp.ansi_buffer.clear();
			break;
		}

		let state = parse(&mut view);
		let len = view.consumed_count();
		match state {
			ANSIState::Valid => {}
			ANSIState::Incomplete => break,
			ANSIState::Invalid => {
				// using an index to avoid double-borrow issues
				for i in 0..len {
					tty.putchar(disp, disp.ansi_buffer.buf[i]);
				}
			}
		}
		disp.ansi_buffer.pop_front(len);
		n += len;
	}
	disp.update_screen();
	n
}

// TODO unit tests
