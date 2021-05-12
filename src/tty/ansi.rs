//! This modules implements the ANSI escape codes for the TTY interface.

use core::cmp::min;
use core::str;
use crate::util::math;
use crate::util;
use crate::vga;
use super::TTY;

/// The character used to initialize ANSI escape sequences.
pub const ESCAPE_CHAR: char = '\x1b';
/// The Control Sequence Introducer character.
const CSI_CHAR: char = '[';

/// The size of the buffer used to parse ANSI escape codes.
pub const BUFFER_SIZE: usize = 16;

/// Buffer storing the current ANSI escape code being handled.
pub struct ANSIBuffer {
	/// The buffer.
	buffer: [u8; BUFFER_SIZE],
	/// The position of the cursor in the buffer.
	cursor: usize,
}

impl ANSIBuffer {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			buffer: [0; BUFFER_SIZE],
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
	/// If more characters are pushed than the remaining capacity, the function truncates the data
	/// to be pushed.
	/// The function returns the number of characters that have been pushed.
	pub fn push(&mut self, buffer: &[u8]) -> usize {
		let len = min(buffer.len(), BUFFER_SIZE - self.cursor);
		for i in 0..len {
			self.buffer[self.cursor + i] = buffer[i];
		}

		self.cursor += len;
		debug_assert!(self.cursor <= BUFFER_SIZE);
		len
	}

	/// Clears the buffer.
	pub fn clear(&mut self) {
		self.cursor = 0;
	}
}

/// Parses the CSI sequence in the given TTY's buffer. If the sequence is complete (either valid or
/// invalid), the function returns `true`. Else, the function returns `false`.
fn parse_csi(tty: &mut TTY) -> bool {
	let nbr_len = util::nbr_len(&tty.ansi_buffer.buffer[2..]);
	if tty.ansi_buffer.len() <= 2 + nbr_len {
		return false;
	}

	let nbr_str = str::from_utf8(&tty.ansi_buffer.buffer[2..nbr_len]);
	if nbr_str.is_err() {
		return false;
	}

	let nbr = str::parse::<i16>(nbr_str.unwrap()).unwrap();

	let final_byte = tty.ansi_buffer.buffer[nbr_len]; // TODO Print remaining
	let status = match final_byte as char {
		'A' => {
			if tty.cursor_y > nbr {
				tty.cursor_y -= nbr;
			}
			true
		},

		'B' => {
			tty.cursor_y = min(tty.cursor_y + nbr, vga::HEIGHT);
			true
		},

		'C' => {
			tty.cursor_x = min(tty.cursor_x + nbr, vga::WIDTH);
			true
		},

		'D' => {
			if tty.cursor_x > nbr {
				tty.cursor_x -= nbr;
			}
			true
		},

		'E' => {
			tty.newline(nbr as _);
			true
		},

		'F' => {
			// TODO
			true
		},

		'G' => {
			tty.cursor_y = math::clamp(nbr, 0, vga::WIDTH);
			true
		},

		'H' => {
			// TODO
			true
		},

		_ => false,
	};
	tty.update();
	status
}

/// Parses the sequence in the given TTY's buffer. If the sequence is complete (either valid or
/// invalid), the function returns `true`. Else, the function returns `false`.
fn parse(tty: &mut TTY) -> bool {
	if tty.ansi_buffer.len() < 2 {
		return false;
	}

	// TODO Check: let first = buffer.buffer[0];
	let second = tty.ansi_buffer.buffer[1];
	match second as char {
		CSI_CHAR => parse_csi(tty),
		// TODO

		_ => false,
	}
}

/// Handles an ANSI escape code stored into buffer `buffer` on the TTY `tty`.
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is undefined.
/// The function returns the number of bytes consumed by the function.
pub fn handle(tty: &mut TTY, buffer: &[u8]) -> usize {
	debug_assert!(!tty.ansi_buffer.is_empty() || buffer[0] == ESCAPE_CHAR as _);
	let n = tty.ansi_buffer.push(buffer);

	if parse(tty) {
		// TODO If invalid, print the buffer's content
		tty.ansi_buffer.clear();
	}

	n
}
