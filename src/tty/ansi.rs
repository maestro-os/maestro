//! This modules implements the ANSI escape codes for the TTY interface.

use super::TTY;
use crate::util;
use crate::vga;
use core::cmp::min;
use core::str;

/// The character used to initialize ANSI escape sequences.
pub const ESCAPE_CHAR: u8 = 0x1b;
/// The Control Sequence Introducer character.
const CSI_CHAR: u8 = b'[';

/// The size of the buffer used to parse ANSI escape codes.
pub const BUFFER_SIZE: usize = 16;

/// Enumeration of possible states of the ANSI parser.
pub enum ANSIState {
	/// The sequence is valid, has been executed and the buffer has been
	/// cleared.
	Valid,
	/// The buffer is waiting for more characters.
	Incomplete,
	/// The sequence is invalid, the content of the buffer has been printed has
	/// normal characters and the buffer has been cleared.
	Invalid,
}

/// Buffer storing the current ANSI escape code being handled.
/// The buffer acts as a FIFO.
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
	/// If more characters are pushed than the remaining capacity, the function
	/// truncates the data to be pushed.
	/// The function returns the number of characters that have been pushed.
	pub fn push_back(&mut self, buffer: &[u8]) -> usize {
		let len = min(buffer.len(), BUFFER_SIZE - self.cursor);
		#[allow(clippy::needless_range_loop)]
		for i in 0..len {
			self.buffer[self.cursor + i] = buffer[i];
		}

		self.cursor += len;
		len
	}

	/// Removes the first `n` characters from the buffer.
	pub fn pop_front(&mut self, n: usize) {
		self.buffer.rotate_left(n);
		self.cursor -= n;
	}
}

/// Converts ANSI color `id` to VGA color.
/// If the given color is invalid, the behaviour is undefined.
fn get_vga_color(id: u8) -> vga::Color {
	match id {
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

/// Moves the cursor on TTY `tty` in the given direction `d`.
/// `n` is the number of cells to travel. If `None`, the default is used (`1`).
fn move_cursor(tty: &mut TTY, d: char, n: Option<i16>) -> ANSIState {
	let n = n.unwrap_or(1);

	match d {
		'A' => {
			if tty.cursor_y > n {
				tty.cursor_y -= n;
			}

			ANSIState::Valid
		}

		'B' => {
			tty.cursor_y = min(tty.cursor_y + n, vga::HEIGHT);

			ANSIState::Valid
		}

		'C' => {
			tty.cursor_x = min(tty.cursor_x + n, vga::WIDTH);

			ANSIState::Valid
		}

		'D' => {
			if tty.cursor_x > n {
				tty.cursor_x -= n;
			}

			ANSIState::Valid
		}

		_ => ANSIState::Invalid,
	}
}

/// Handles an Select Graphics Renderition (SGR) command.
/// `command` is the id of the command. If `None`, the default is used (`0`).
fn parse_sgr(tty: &mut TTY, command: Option<i16>) -> ANSIState {
	let command = command.unwrap_or(0);

	match command {
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

		30..=37 | 90..=97 => {
			tty.set_fgcolor(get_vga_color(command as _));
			ANSIState::Valid
		}

		38 => {
			// TODO Set fg color
			ANSIState::Valid
		}

		39 => {
			tty.reset_fgcolor();
			ANSIState::Valid
		}

		40..=47 | 100..=107 => {
			tty.set_bgcolor(get_vga_color(command as _));
			ANSIState::Valid
		}

		48 => {
			// TODO Set bg color
			ANSIState::Valid
		}

		49 => {
			tty.reset_bgcolor();
			ANSIState::Valid
		}

		50..=107 => ANSIState::Valid,

		_ => ANSIState::Invalid,
	}
}

/// Parses the CSI sequence in the given TTY's buffer.
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse_csi(tty: &mut TTY) -> (ANSIState, usize) {
	let nbr_len = util::nbr_len(&tty.ansi_buffer.buffer[2..]);
	if tty.ansi_buffer.len() <= 2 + nbr_len {
		return (ANSIState::Incomplete, 0);
	}

	let nbr_str = str::from_utf8(&tty.ansi_buffer.buffer[2..(2 + nbr_len)]);
	if nbr_str.is_err() {
		return (ANSIState::Invalid, 0);
	}

	let nbr = str::parse::<i16>(nbr_str.unwrap()).ok();

	let final_byte = tty.ansi_buffer.buffer[2 + nbr_len];
	let status = match final_byte as char {
		'A' | 'B' | 'C' | 'D' => move_cursor(tty, final_byte as char, nbr),

		'E' => {
			tty.newline(nbr.unwrap_or(1) as _);
			ANSIState::Valid
		}

		'F' => {
			// TODO Previous line
			ANSIState::Valid
		}

		'G' => {
			tty.cursor_y = nbr.unwrap_or(1).clamp(0, vga::WIDTH);
			ANSIState::Valid
		}

		'H' => {
			// TODO Set cursor position
			ANSIState::Valid
		}

		'J' => {
			// TODO Erase in display
			ANSIState::Valid
		}

		'K' => {
			// TODO Erase in line
			ANSIState::Valid
		}

		'S' => {
			// TODO Scroll up
			ANSIState::Valid
		}

		'T' => {
			// TODO Scroll down
			ANSIState::Valid
		}

		'm' => parse_sgr(tty, nbr),

		_ => ANSIState::Invalid,
	};

	tty.update();
	(status, 2 + nbr_len + 1)
}

/// Parses the sequence in the given TTY's buffer.
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse(tty: &mut TTY) -> (ANSIState, usize) {
	if tty.ansi_buffer.len() < 2 {
		return (ANSIState::Incomplete, 0);
	}

	if tty.ansi_buffer.buffer[0] != ESCAPE_CHAR {
		return (ANSIState::Invalid, 0);
	}

	match tty.ansi_buffer.buffer[1] {
		CSI_CHAR => parse_csi(tty),
		// TODO
		_ => (ANSIState::Invalid, 0),
	}
}

/// Handles an ANSI escape code stored into buffer `buffer` on the TTY `tty`.
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is
/// undefined. The function returns the number of bytes consumed by the
/// function.
pub fn handle(tty: &mut TTY, buffer: &[u8]) -> usize {
	if tty.ansi_buffer.is_empty() || buffer[0] != ESCAPE_CHAR as _ {
		return 0;
	}

	let n = tty.ansi_buffer.push_back(buffer);

	while !tty.ansi_buffer.is_empty() {
		let (state, len) = parse(tty);

		match state {
			ANSIState::Valid => {
				tty.ansi_buffer.pop_front(len);
				tty.update();
			}

			ANSIState::Incomplete => break,

			ANSIState::Invalid => {
				let mut i = 0;
				while i < tty.ansi_buffer.buffer.len() {
					let c = tty.ansi_buffer.buffer[i];
					if c == ESCAPE_CHAR {
						break;
					}

					tty.putchar(c);
					i += 1;
				}

				tty.ansi_buffer.pop_front(i);
				tty.update();
			}
		}
	}

	n
}
