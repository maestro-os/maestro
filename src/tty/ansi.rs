//! ANSI escape sequences allow to control the terminal by specifying commands in standard output of the terminal.

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
pub(super) enum ANSIState {
	/// The sequence is valid has been executed and has been pruned from the buffer.
	Valid,
	/// The sequence is incomplete. Waiting for more data.
	Incomplete,
	/// The sequence is invalid, it has been printed as normal characters and has been pruned from the buffer.
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
		self.tty.ansi_buffer.buf[self.cursor..].is_empty()
	}

	/// Returns the number of consumed characters.
	fn consumed_count(&self) -> usize {
		self.cursor
	}

	/// Consumes the next single character of the view.
	///
	/// If not enough data remains, the function returns `None`.
	fn next_char(&mut self) -> Option<u8> {
		let c = *self.buffer().get(self.cursor)?;
		self.cursor += 1;
		Some(c)
	}

	/// Consumes the next number of the view.
	///
	/// If not enough data remains or if the number is invalid, the function returns `None`.
	fn next_nbr(&mut self) -> Option<u32> {
		let nbr_len = util::nbr_len(&self.buffer()[self.cursor..]);
		// FIXME: doesn't work on invalid UTF-8. use a custom parsing function
		let Ok(nbr) = str::from_utf8(&self.buffer()[self.cursor..(self.cursor + nbr_len)]) else {
			return None;
		};
		let n = str::parse::<u32>(nbr).ok()?;
		self.cursor += nbr_len;
		Some(n)
	}
}

/// Converts ANSI color `id` to VGA color.
///
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

/// Moves the cursor on TTY `tty`.
///
/// Arguments:
/// - `d` is the direction character.
/// - `n` is the number of cells to travel. If `None`, the default is used (`1`).
fn move_cursor(tty: &mut TTY, d: u8, n: Option<u16>) -> ANSIState {
	let n = n.unwrap_or(1) as i16;
	match d {
		b'A' => {
			if tty.cursor_y > n {
				tty.cursor_y -= n;
			}
			ANSIState::Valid
		}
		b'B' => {
			tty.cursor_y = min(tty.cursor_y + n, vga::HEIGHT);
			ANSIState::Valid
		}
		b'C' => {
			tty.cursor_x = min(tty.cursor_x + n, vga::WIDTH);
			ANSIState::Valid
		}
		b'D' => {
			if tty.cursor_x > n {
				tty.cursor_x -= n;
			}
			ANSIState::Valid
		}
		_ => ANSIState::Invalid,
	}
}

/// Handles an Select Graphics Renderition (SGR) command.
///
/// `command` is the id of the command. If `None`, the default is used (`0`).
fn parse_sgr(tty: &mut TTY, command: Option<u32>) -> ANSIState {
	match command.unwrap_or(0) {
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
			tty.set_fgcolor(get_vga_color(c as _));
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
		c @ (40..=47 | 100..=107) => {
			tty.set_bgcolor(get_vga_color(c as _));
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

/// Parses the CSI sequence in the given buffer view.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse_csi(view: &mut ANSIBufferView) -> (ANSIState, usize) {
	let nbr = view.next_nbr();
	let Some(cmd) = view.next_char() else {
		return (ANSIState::Incomplete, 0);
	};
	let status = match cmd {
		b'?' => match (view.next_nbr(), view.next_char()) {
			(Some(7 | 25), Some(b'h')) => {
				view.tty().set_cursor_visible(true);
				ANSIState::Valid
			},
			(Some(7 | 25), Some(b'l')) => {
				view.tty().set_cursor_visible(true);
				ANSIState::Valid
			},
			_ => ANSIState::Invalid,
		},
		b'A'..=b'D' => move_cursor(view.tty(), cmd, nbr.map(|i| i as _)),
		b'E' => {
			view.tty().newline(nbr.unwrap_or(1) as _);
			ANSIState::Valid
		}
		b'F' => {
			// TODO Previous line
			ANSIState::Valid
		}
		b'G' => {
			view.tty().cursor_y = nbr.map(|i| i as i16).unwrap_or(1).clamp(0, vga::WIDTH);
			ANSIState::Valid
		}
		b'H' => {
			// TODO Set cursor position
			ANSIState::Valid
		}
		b'J' => {
			// TODO Erase in display
			ANSIState::Valid
		}
		b'K' => {
			// TODO Erase in line
			ANSIState::Valid
		}
		b'S' => {
			// TODO Scroll up
			ANSIState::Valid
		}
		b'T' => {
			// TODO Scroll down
			ANSIState::Valid
		}
		b'm' => parse_sgr(view.tty(), nbr),
		_ => ANSIState::Invalid,
	};

	view.tty().update();
	(status, view.consumed_count())
}

/// Parses the sequence in the given TTY's buffer.
///
/// The function returns the state of the sequence. If valid, the length of the
/// sequence is also returned.
fn parse(tty: &mut TTY) -> (ANSIState, usize) {
	let mut view = ANSIBufferView::new(tty);
	if view.next_char() != Some(ESCAPE_CHAR) {
		return (ANSIState::Invalid, view.consumed_count());
	}
	let Some(prefix) = view.next_char() else { return (ANSIState::Invalid, 0); };

	match prefix {
		CSI_CHAR => parse_csi(&mut view),
		// TODO
		_ => (ANSIState::Invalid, view.consumed_count()),
	}
}

/// Handles an ANSI escape sequences stored into the buffer `buffer` on the TTY `tty`.
///
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is
/// undefined.
///
/// The function returns the number of bytes consumed by the function.
pub fn handle(tty: &mut TTY, buffer: &[u8]) -> usize {
	if tty.ansi_buffer.is_empty() && buffer[0] != ESCAPE_CHAR as _ {
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
				// using an index to avoid double-borrow issues
				for i in 0..len {
					tty.putchar(tty.ansi_buffer.buf[i]);
				}
				tty.ansi_buffer.pop_front(len);
				tty.update();
			}
		}
	}
	n
}
