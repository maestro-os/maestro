/// When booting, the kernel can take command line arguments. This module implements a parse for
/// these arguments.

use core::str;

/// Command line argument parser.
/// Every bytes in the command line are interpreted as ASCII characters.
pub struct ArgsParser<'a> {
	/// The command line.
	cmdline: &'a str,

	/// The root device major number.
	root_major: u32,
	/// The root device minor number.
	root_minor: u32,

	/// The offset of the beginning of the init path.
	init_begin: usize,
	/// The size of the init path. If not specified, the value is zero.
	init_size: usize,

	/// Whether the kernel boots silently.
	silent: bool,
}

impl<'a> ArgsParser<'a> {
	/// Returns a string containing the next argument and the next offset.
	/// `off` is the offset of the beginning of the argument.
	/// If no argument is remaining, the function returns None.
	fn next_arg(cmdline: &str, mut off: usize) -> Option<(&str, usize)> {
		let slice = cmdline.as_bytes();
		while off < slice.len() && slice[off] as char == ' ' {
			off += 1;
		}

		if off < slice.len() {
			let remaining = &slice[off..];
			let mut i = 0;
			while i < remaining.len() && remaining[i] as char != ' ' {
				i += 1;
			}

			Some((unsafe {
				str::from_utf8_unchecked(&remaining[0..i]) // TODO Use the safe version and print an error if invalid?
			}, off + i))
		} else {
			None
		}
	}

	/// Parses the given command line and returns a new instance.
	pub fn parse(cmdline: &'a str) -> Result<Self, &'static str> {
		let mut off = 0;

		let mut root_major = 0;
		let mut root_minor = 0;
		let mut init_begin = 0;
		let mut init_size = 0;
		let mut silent = false;

		loop {
			let r = Self::next_arg(cmdline, off);
			if r.is_none() {
				break;
			}
			let (arg, next_off) = r.unwrap();
			off = next_off;

			match arg {
				"-root" => {
					let (arg, next_off) = Self::next_arg(cmdline, off)
						.ok_or("Missing major number")?;
					off = next_off;
					root_major = arg.parse::<u32>().unwrap(); // TODO Return error on fail

					let (arg, next_off) = Self::next_arg(cmdline, off)
						.ok_or("Missing minor number")?;
					off = next_off;
					root_minor = arg.parse::<u32>().unwrap(); // TODO Return error on fail
				},
				"-init" => {
					// TODO Parse the next argument
					init_begin = 0;
					init_size = 0;
				},
				"-silent" => silent = true,

				_ => {
					// TODO Find a way to pass the argument in the message
					return Err("Unknown command line argument");
				}
			}
		}

		// TODO Print an error if `-root` wasn't specified

		Ok(Self {
			cmdline: cmdline,

			root_major: root_major,
			root_minor: root_minor,

			init_begin: init_begin,
			init_size: init_size,

			silent: silent,
		})
	}

	/// Returns the major and minor numbers of the root device.
	pub fn get_root_dev(&self) -> (u32, u32) {
		(self.root_major, self.root_minor)
	}

	/// Returns the init binary path if specified.
	pub fn get_init_path(&self) -> Option<&'a str> {
		if self.init_size != 0 {
			let begin = self.init_begin;
			let end = begin + self.init_size;
			let slice = &self.cmdline.as_bytes()[begin..end];

			Some(unsafe {
				str::from_utf8_unchecked(slice) // TODO Use the safe version and print an error if invalid?

			})
		} else {
			None
		}
	}

	/// If true, the kernel doesn't print logs while booting.
	pub fn is_silent(&self) -> bool {
		self.silent
	}
}
