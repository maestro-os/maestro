//! When booting, the kernel can take command line arguments. This module
//! implements a parse for these arguments.

use crate::util::DisplayableStr;
use crate::vga;
use core::cmp::min;
use core::fmt;
use core::str;

/// Skips spaces in slice `slice`, starting at offset `i`.
fn skip_spaces(slice: &[u8], i: &mut usize) {
	let mut j = *i;

	while j < slice.len() && (slice[j] as char).is_ascii_whitespace() {
		j += 1;
	}

	*i = j;
}

/// Parses the number represented by the string in the given slice.
///
/// If the slice doesn't contain a valid number, the function returns `None`.
fn parse_nbr(slice: &[u8]) -> Option<u32> {
	str::from_utf8(slice).ok().and_then(|s| s.parse().ok())
}

/// Structure representing a command line parsing error.
#[derive(Debug)]
pub struct ParseError<'s> {
	/// The command line.
	cmdline: &'s [u8],
	/// An error message.
	err: &'static str,

	/// The offset and size of the token that caused the error.
	token: Option<(usize, usize)>,
}

impl<'s> ParseError<'s> {
	/// Creates a new instance.
	pub fn new(cmdline: &'s [u8], err: &'static str, token: Option<(usize, usize)>) -> Self {
		Self {
			cmdline,
			err,

			token,
		}
	}
}

impl<'s> fmt::Display for ParseError<'s> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		write!(
			fmt,
			"Error while parsing command line arguments: {}",
			self.err
		)?;

		let Some((begin, size)) = self.token else {
            return Ok(());
		};

		let mut i = 0;
		while i < self.cmdline.len() {
			let l = min(self.cmdline.len() - i, vga::WIDTH as usize - 1);
			write!(fmt, "{}", DisplayableStr(&self.cmdline[i..(i + l)]))?;

			let mut j = i;
			while j < i + l {
				if j == begin {
					write!(fmt, "^")?;
				} else if j > begin && j < begin + size {
					write!(fmt, "-")?;
				} else {
					write!(fmt, " ")?;
				}

				j += 1;
			}
			writeln!(fmt)?;

			i += vga::WIDTH as usize - 1;
		}

		writeln!(fmt)
	}
}

/// Structure representing a token in the command line.
struct Token<'s> {
	/// The token's string.
	s: &'s [u8],
	/// The offset to the beginning of the token in the command line.
	begin: usize,
}

struct TokenIterator<'s> {
	/// The string to iterate on.
	s: &'s [u8],
	/// The current index on the string.
	i: usize,
}

impl<'s> Iterator for TokenIterator<'s> {
	type Item = Token<'s>;

	fn next(&mut self) -> Option<Self::Item> {
		skip_spaces(&self.s, &mut self.i);
		let mut j = self.i;
		while j < self.s.len() && !(self.s[j] as char).is_ascii_whitespace() {
			j += 1;
		}

		if j > self.i {
			let tok = Token {
				s: &self.s[self.i..j],
				begin: self.i,
			};
			self.i = j;

			Some(tok)
		} else {
			None
		}
	}
}

/// Command line argument parser.
///
/// Every bytes in the command line are interpreted as ASCII characters.
pub struct ArgsParser<'s> {
	/// The root device major and minor numbers.
	root: Option<(u32, u32)>,
	/// The path to the init binary, if specified.
	init: Option<&'s [u8]>,
	/// Whether the kernel boots silently.
	silent: bool,
}

impl<'s> ArgsParser<'s> {
	/// Parses the given command line and returns a new instance.
	pub fn parse(cmdline: &'s [u8]) -> Result<Self, ParseError<'_>> {
		let mut s = Self {
			root: None,
			init: None,
			silent: false,
		};

		let mut iter = TokenIterator {
			s: cmdline,
			i: 0,
		}
		.enumerate();
		loop {
			let Some((i, token)) = iter.next() else {
                break;
            };

			match token.s {
				b"-root" => {
					let (Some((_, major)), Some((_, minor))) = (iter.next(), iter.next()) else {
                        return Err(ParseError::new(
                            cmdline,
                            "not enough arguments for `-root`",
                            Some((token.begin, token.s.len())),
                        ));
                    };

					let Some(major) = parse_nbr(major.s) else {
						return Err(ParseError::new(
							cmdline,
							"invalid major number",
							Some((i + 1, 1)),
						));
					};
					let Some(minor) = parse_nbr(minor.s) else {
						return Err(ParseError::new(
							cmdline,
							"invalid minor number",
							Some((i + 2, 1)),
						));
					};

					s.root = Some((major, minor));
				}

				b"-init" => {
					let Some((_, init)) = iter.next() else {
						return Err(ParseError::new(
							cmdline,
							"not enough arguments for `-init`",
							Some((token.begin, token.s.len())),
						));
                    };

					s.init = Some(init.s);
				}

				b"-silent" => s.silent = true,

				_ => {
					return Err(ParseError::new(
						cmdline,
						"invalid argument",
						Some((token.begin, token.s.len())),
					));
				}
			}
		}

		Ok(s)
	}

	/// Returns the major and minor numbers of the root device.
	pub fn get_root_dev(&self) -> Option<(u32, u32)> {
		self.root
	}

	/// Returns the init binary path if specified.
	pub fn get_init_path(&self) -> Option<&'s [u8]> {
		self.init
	}

	/// If `true`, the kernel doesn't print logs while booting.
	pub fn is_silent(&self) -> bool {
		self.silent
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn cmdline0() {
		assert!(ArgsParser::parse(b"-bleh").is_err());
	}

	#[test_case]
	fn cmdline1() {
		assert!(ArgsParser::parse(b"-root -bleh").is_err());
	}

	#[test_case]
	fn cmdline2() {
		assert!(ArgsParser::parse(b"-root 1 0 -bleh").is_err());
	}

	#[test_case]
	fn cmdline3() {
		assert!(ArgsParser::parse(b"-root 1 0").is_ok());
	}

	#[test_case]
	fn cmdline4() {
		assert!(ArgsParser::parse(b"-root 1 0 -silent").is_ok());
	}

	#[test_case]
	fn cmdline5() {
		assert!(ArgsParser::parse(b"-root 1 0 -init").is_err());
	}

	#[test_case]
	fn cmdline6() {
		assert!(ArgsParser::parse(b"-root 1 0 -init bleh").is_ok());
	}

	#[test_case]
	fn cmdline7() {
		assert!(ArgsParser::parse(b"-root 1 0 -init bleh -silent").is_ok());
	}
}
