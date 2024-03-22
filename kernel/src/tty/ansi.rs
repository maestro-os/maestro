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

use super::TTY;
use crate::tty::vga;
use core::{
	cmp::{max, min},
	str,
};

/// The character used to initialize ANSI escape sequences.
pub const ESCAPE_CHAR: u8 = 0x1b;
/// The Control Sequence Introducer character.
const CSI_CHAR: u8 = b'[';

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
	pub fn new() -> Self {
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
	tty: &'tty mut TTY,
	/// The current offset of the view in the buffer.
	cursor: usize,
}

impl<'tty> ANSIBufferView<'tty> {
	/// Creates a view the buffer of the given TTY.
	fn new(tty: &'tty mut TTY) -> Self {
		Self {
			tty,
			cursor: 0,
		}
	}

	/// Returns the associated TTY.
	pub fn tty(&mut self) -> &mut TTY {
		self.tty
	}

	/// Returns an immutable reference to the underlying buffer view.
	pub fn buffer(&self) -> &[u8] {
		&self.tty.ansi_buffer.buf[..self.tty.ansi_buffer.cursor]
	}

	/// Tells whether the view is empty.
	fn is_empty(&self) -> bool {
		self.buffer()[self.cursor..].is_empty()
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
	/// A successful return doesn't necessarily means the number is complete. The buffer might be
	/// incomplete and need more data.
	///
	/// If not enough data remains or if the number is invalid, the function returns `None`.
	fn next_nbr(&mut self) -> Option<u32> {
		let nbr_len = utils::nbr_len(&self.buffer()[self.cursor..]);
		let Ok(nbr) = str::from_utf8(&self.buffer()[self.cursor..(self.cursor + nbr_len)]) else {
			return None;
		};
		let n = str::parse::<u32>(nbr).ok()?;
		self.cursor += nbr_len;
		Some(n)
	}

	/// Consumes the next sequence of `;`-separated numbers.
	///
	/// The function takes a buffer to write the sequence on. If the buffer is not large enough to
	/// fit the whole sequence, it is truncated.
	fn next_nbr_sequence<'b>(&mut self, buf: &'b mut [Option<u32>]) -> &'b [Option<u32>] {
		let mut i = 0;
		for b in buf.iter_mut() {
			*b = self.next_nbr();
			i += 1;

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

		&buf[..i]
	}
}

/// Returns the VGA color associated with the given command.
fn get_vga_color_from_cmd(cmd: u8) -> vga::Color {
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
fn get_vga_color_from_id(id: u8) -> vga::Color {
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
fn move_cursor(tty: &mut TTY, d: u8, n: Option<u32>) -> ANSIState {
	let n = n.unwrap_or(1).clamp(0, i16::MAX as _) as i16;
	match d {
		b'A' => {
			let n = tty.cursor_y.checked_sub(n).unwrap_or(0);
			tty.cursor_y = max(n, 0);
			ANSIState::Valid
		}
		b'B' => {
			let n = tty.cursor_y.checked_add(n).unwrap_or(vga::HEIGHT - 1);
			tty.cursor_y = min(n, vga::HEIGHT - 1);
			ANSIState::Valid
		}
		b'C' => {
			let n = tty.cursor_x.checked_add(n).unwrap_or(vga::WIDTH - 1);
			tty.cursor_x = min(n, vga::WIDTH - 1);
			ANSIState::Valid
		}
		b'D' => {
			let n = tty.cursor_x.checked_sub(n).unwrap_or(0);
			tty.cursor_x = max(n, 0);
			ANSIState::Valid
		}
		_ => ANSIState::Invalid,
	}
}

/// Handles an Select Graphics Renderition (SGR) command.
///
/// `seq` is the id of the numbers describing the command.
fn parse_sgr(tty: &mut TTY, seq: &[Option<u32>]) -> ANSIState {
	match seq.first().cloned().flatten().unwrap_or(0) {
		0 => {
			tty.reset_attrs();
			ANSIState::Valid
		}
		1 => ANSIState::Valid, // TODO Bold
		2 => ANSIState::Valid, // TODO Faint
		3 => ANSIState::Valid, // TODO Italic
		4 => ANSIState::Valid, // TODO Underline
		5 | 6 => {
			tty.set_blinking(true);
			ANSIState::Valid
		}
		7 => {
			tty.swap_colors();
			ANSIState::Valid
		}
		8 => ANSIState::Valid,
		9 => ANSIState::Valid,  // TODO Crossed-out
		10 => ANSIState::Valid, // TODO Primary font
		11 => ANSIState::Valid, // TODO Alternative font
		12 => ANSIState::Valid, // TODO Alternative font
		13 => ANSIState::Valid, // TODO Alternative font
		14 => ANSIState::Valid, // TODO Alternative font
		15 => ANSIState::Valid, // TODO Alternative font
		16 => ANSIState::Valid, // TODO Alternative font
		17 => ANSIState::Valid, // TODO Alternative font
		18 => ANSIState::Valid, // TODO Alternative font
		19 => ANSIState::Valid, // TODO Alternative font
		20 | 21 => ANSIState::Valid,
		22 => ANSIState::Valid, // TODO Normal intensity
		23 => ANSIState::Valid, // TODO Not italic
		24 => ANSIState::Valid, // TODO Not underlined
		25 => {
			tty.set_blinking(false);
			ANSIState::Valid
		}
		26 => ANSIState::Valid,
		27 => ANSIState::Valid, // TODO Not reversed
		28 => ANSIState::Valid,
		29 => ANSIState::Valid, // TODO Not crossed-out
		c @ (30..=37 | 90..=97) => {
			tty.set_fgcolor(get_vga_color_from_cmd(c as _));
			ANSIState::Valid
		}
		38 => match seq.get(1).cloned().flatten() {
			Some(2) => {
				// TODO with VGA, use closest color
				ANSIState::Invalid
			}
			Some(5) => {
				let Some(nbr) = seq.get(2).cloned().flatten() else {
					return ANSIState::Invalid;
				};
				tty.set_fgcolor(get_vga_color_from_id(nbr as _));
				ANSIState::Valid
			}
			_ => ANSIState::Invalid,
		},
		39 => {
			tty.reset_fgcolor();
			ANSIState::Valid
		}
		c @ (40..=47 | 100..=107) => {
			tty.set_bgcolor(get_vga_color_from_cmd(c as _));
			ANSIState::Valid
		}
		48 => match seq.get(1).cloned().flatten() {
			Some(2) => {
				// TODO with VGA, use closest color
				ANSIState::Invalid
			}
			Some(5) => {
				let Some(nbr) = seq.get(2).cloned().flatten() else {
					return ANSIState::Invalid;
				};
				tty.set_bgcolor(get_vga_color_from_id(nbr as _));
				ANSIState::Valid
			}
			_ => ANSIState::Invalid,
		},
		49 => {
			tty.reset_bgcolor();
			ANSIState::Valid
		}
		50..=107 => ANSIState::Valid,
		_ => ANSIState::Invalid,
	}
}

/// Parses the CSI sequence in the given buffer view.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse_csi(view: &mut ANSIBufferView) -> ANSIState {
	let mut seq_buf: [Option<u32>; SEQ_MAX] = [None; SEQ_MAX];
	let seq = view.next_nbr_sequence(seq_buf.as_mut_slice());
	let Some(cmd) = view.next_char() else {
		return ANSIState::Incomplete;
	};

	let status = match (seq, cmd) {
		(_, b'?') => match (view.next_nbr(), view.next_char()) {
			(Some(7 | 25), Some(b'h')) => {
				view.tty().set_cursor_visible(true);
				ANSIState::Valid
			}
			(Some(7 | 25), Some(b'l')) => {
				view.tty().set_cursor_visible(false);
				ANSIState::Valid
			}
			_ => ANSIState::Invalid,
		},
		(&[nbr], b'A'..=b'D') => move_cursor(view.tty(), cmd, nbr.map(|i| i as _)),
		(&[nbr], b'E') => {
			view.tty().newline(nbr.unwrap_or(1) as _);
			ANSIState::Valid
		}
		(&[_nbr], b'F') => {
			// TODO Previous line
			ANSIState::Valid
		}
		(&[nbr], b'G') => {
			view.tty().cursor_y = nbr.map(|i| i as _).unwrap_or(1).clamp(1, vga::WIDTH + 1) - 1;
			ANSIState::Valid
		}
		(&[row, column], b'H') => {
			view.tty().cursor_x = column.map(|i| i as _).unwrap_or(1).clamp(1, vga::WIDTH + 1) - 1;
			view.tty().cursor_y = row.map(|i| i as _).unwrap_or(1).clamp(1, vga::HEIGHT + 1) - 1;
			ANSIState::Valid
		}
		(&[_nbr], b'J') => {
			// TODO Erase in display
			ANSIState::Valid
		}
		(&[_nbr], b'K') => {
			// TODO Erase in line
			ANSIState::Valid
		}
		(&[_nbr], b'S') => {
			// TODO Scroll up
			ANSIState::Valid
		}
		(&[_nbr], b'T') => {
			// TODO Scroll down
			ANSIState::Valid
		}
		(seq, b'm') => parse_sgr(view.tty(), seq),
		_ => ANSIState::Invalid,
	};
	view.tty().update();
	status
}

/// Parses the sequence in the given buffer.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse(view: &mut ANSIBufferView) -> ANSIState {
	if view.next_char() != Some(ESCAPE_CHAR) {
		return ANSIState::Invalid;
	}
	let Some(prefix) = view.next_char() else {
		return ANSIState::Incomplete;
	};

	match prefix {
		CSI_CHAR => parse_csi(view),
		// TODO
		_ => ANSIState::Invalid,
	}
}

/// Handles an ANSI escape sequences stored into the buffer `buffer` on the TTY `tty`.
///
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is
/// undefined.
///
/// The function returns the number of bytes consumed by the function.
pub fn handle(tty: &mut TTY, buffer: &[u8]) -> usize {
	tty.ansi_buffer.push_back(buffer);
	let mut n = 0;
	while !tty.ansi_buffer.is_empty() {
		let mut view = ANSIBufferView::new(tty);
		if view.peek_char() != Some(ESCAPE_CHAR) {
			tty.ansi_buffer.clear();
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
					tty.putchar(tty.ansi_buffer.buf[i]);
				}
			}
		}
		tty.ansi_buffer.pop_front(len);
		n += len;
	}
	tty.update();
	n
}

// TODO unit tests
