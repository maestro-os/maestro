//! When booting, the kernel can take command line arguments. This module implements a parse for
//! these arguments.

use core::cmp::min;
use core::str;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::vga;

/// Command line argument parser.
/// Every bytes in the command line are interpreted as ASCII characters.
pub struct ArgsParser {
	/// The root device major number.
	root_major: u32,
	/// The root device minor number.
	root_minor: u32,

	/// The path to the init binary, if specified.
	init: Option<String>,

	/// Whether the kernel boots silently.
	silent: bool,
}

/// Structure representing a token in the command line.
struct Token {
	/// The token's string.
	s: String,
	/// The offset to the beginning of the token in the command line.
	begin: usize,
}

impl Token {
	/// Returns the length of the token.
	pub fn len(&self) -> usize {
		self.s.as_str().as_bytes().len()
	}
}

/// Structure representing a command line parsing error.
#[derive(Debug)]
pub struct ParseError<'a> {
	/// The command line.
	cmdline: &'a str,
	/// An error message.
	err: &'static str,

	/// The offset and size of the token that caused the error.
	token: Option<(usize, usize)>,
}

impl<'a> ParseError<'a> {
	/// Creates a new instance.
	pub fn new(cmdline: &'a str, err: &'static str, token: Option<(usize, usize)>) -> Self {
		Self {
			cmdline,
			err,

			token,
		}
	}

	/// Prints the print error.
	pub fn print(&self) {
		crate::println!("Error while parsing command line arguments: {}", self.err);

		if let Some((begin, size)) = self.token {
			let slice = &self.cmdline.as_bytes();
			let cmdline_len = slice.len();
			let mut i = 0;
			while i < cmdline_len {
				let l = min(cmdline_len - i, vga::WIDTH as usize - 1);
				let s = unsafe {
					str::from_utf8_unchecked(&slice[i..(i + l)])
				};
				crate::println!("{}", s);

				let mut j = i;
				while j < i + l {
					if j == begin {
						crate::print!("^");
					} else if j > begin && j < begin + size {
						crate::print!("-");
					} else {
						crate::print!(" ");
					}

					j += 1;
				}
				crate::println!();

				i += vga::WIDTH as usize - 1;
			}

			crate::println!();
		}
	}
}

impl ArgsParser {
	/// Returns `true` if the given character `c` is a whitespace.
	fn is_space(c: char) -> bool {
		c == ' ' || c == '\n' || c == '\t'
	}

	/// Skips spaces in slice `slice`, starting at offset `i`.
	fn skip_spaces(slice: &[u8], i: &mut usize) {
		let mut j = *i;

		while j < slice.len() && Self::is_space(slice[j] as char) {
			j += 1;
		}

		*i = j;
	}

	/// Creates a new token starting a the given offset `i` in the given command line `cmdline`.
	fn new_token<'a>(cmdline: &'a str, i: &mut usize) -> Result<Option<Token>, ParseError<'a>> {
		let slice = &cmdline.as_bytes();

		Self::skip_spaces(slice, i);
		let mut j = *i;
		while j < slice.len() && !Self::is_space(slice[j] as char) {
			j += 1;
		}

		if j > *i {
			if let Ok(s) = String::from(unsafe {
					str::from_utf8_unchecked(&slice[*i..j])
				}) {
				let tok = Token {
					s,
					begin: *i,
				};
				*i = j;

				Ok(Some(tok))
			} else {
				Err(ParseError::new(cmdline, "Out of memory", None))
			}
		} else {
			Ok(None)
		}
	}

	/// Tokenizes the command line arguments and returns an array containing all the tokens.
	/// Every characters are interpreted as ASCII characters. If a non-ASCII character is passed,
	/// the function returns an error.
	fn tokenize(cmdline: &str) -> Result<Vec<Token>, ParseError> {
		let mut tokens = Vec::new();
		let mut i = 0;

		while i < cmdline.len() {
			if let Some(tok) = Self::new_token(cmdline, &mut i)? {
				if tokens.push(tok).is_err() {
					return Err(ParseError::new(cmdline, "Out of memory", None));
				}
			}
		}

		Ok(tokens)
	}

	/// Parses the given command line and returns a new instance.
	pub fn parse(cmdline: &str) -> Result<Self, ParseError<'_>> {
		let mut s = Self {
			root_major: 0,
			root_minor: 0,

			init: None,

			silent: false,
		};

		let mut root_specified = false;

		let tokens = Self::tokenize(cmdline)?;
		let mut i = 0;
		while i < tokens.len() {
			let token_str = tokens[i].s.as_str();

			match token_str {
				"-root" => {
					if tokens.len() < i + 3 {
						let begin = tokens[i].begin;
						let size = tokens[i].len();
						return Err(ParseError::new(cmdline, "Not enough arguments for `-root`",
							Some((begin, size))));
					}

					s.root_major = tokens[i + 1].s.as_str().parse::<u32>().unwrap(); // TODO Return error on fail
					s.root_minor = tokens[i + 2].s.as_str().parse::<u32>().unwrap(); // TODO Return error on fail

					i += 3;
					root_specified = true;
				},

				"-init" => {
					if tokens.len() < i + 2 {
						let begin = tokens[i].begin;
						let size = tokens[i].len();
						return Err(ParseError::new(cmdline, "Not enough arguments for `-init`",
							Some((begin, size))));
					}

					if let Ok(init) = tokens[i + 1].s.failable_clone() {
						s.init = Some(init);
					} else {
						return Err(ParseError::new(cmdline, "Out of memory", None));
					}

					i += 2;
				},

				"-silent" => {
					s.silent = true;

					i += 1;
				},

				_ => {
					let begin = tokens[i].begin;
					let size = tokens[i].len();
					return Err(ParseError::new(cmdline, "Invalid argument", Some((begin, size))));
				}
			}
		}

		if !root_specified {
			return Err(ParseError::new(cmdline, "`-root` not specified", None));
		}

		Ok(s)
	}

	/// Returns the major and minor numbers of the root device.
	pub fn get_root_dev(&self) -> (u32, u32) {
		(self.root_major, self.root_minor)
	}

	/// Returns the init binary path if specified.
	pub fn get_init_path(&self) -> &Option<String> {
		&self.init
	}

	/// If true, the kernel doesn't print logs while booting.
	pub fn is_silent(&self) -> bool {
		self.silent
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn cmdline0() {
		assert!(ArgsParser::parse("").is_err());
	}

	#[test_case]
	fn cmdline1() {
		assert!(ArgsParser::parse("-bleh").is_err());
	}

	#[test_case]
	fn cmdline2() {
		assert!(ArgsParser::parse("-root -bleh").is_err());
	}

	#[test_case]
	fn cmdline3() {
		assert!(ArgsParser::parse("-root 1 0 -bleh").is_err());
	}

	#[test_case]
	fn cmdline4() {
		assert!(ArgsParser::parse("-root 1 0").is_ok());
	}

	#[test_case]
	fn cmdline5() {
		assert!(ArgsParser::parse("-root 1 0 -silent").is_ok());
	}

	#[test_case]
	fn cmdline6() {
		assert!(ArgsParser::parse("-root 1 0 -init").is_err());
	}

	#[test_case]
	fn cmdline7() {
		assert!(ArgsParser::parse("-root 1 0 -init bleh").is_ok());
	}

	#[test_case]
	fn cmdline8() {
		assert!(ArgsParser::parse("-root 1 0 -init bleh -silent").is_ok());
	}
}
