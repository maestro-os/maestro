/// This modules implements the ANSI escape codes for the TTY interface.

use super::TTY;

/// The character used to initialize ANSI escape sequences.
pub const ESCAPE_CHAR: char = '\x1b';

/// Handles an ANSI escape code stored into buffer `buffer` on the TTY `tty`.
/// If the buffer doesn't begin with the ANSI escape character, the behaviour is undefined.
/// The function returns the number of bytes consumed by the function.
pub fn handle(_tty: &mut TTY, _buffer: &str) -> usize {
	// TODO

	0
}
