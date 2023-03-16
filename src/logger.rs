//! This modules handles kernel logging.
//! If the logger is silent, it will not print the logs on the screen but it
//! will keep it in memory anyways.

use crate::tty;
use crate::util::lock::IntMutex;
use core::cmp::min;
use core::fmt::Write;

/// The size of the kernel logs buffer in bytes.
const LOGS_SIZE: usize = 1048576;

/// The kernel's logger.
static LOGGER: IntMutex<Logger> = IntMutex::new(Logger::new());

/// Initializes logging.
/// `silent` tells whether the logger is silent.
pub fn init(silent: bool) {
	LOGGER.lock().set_silent(silent);
}

/// Returns a mutable reference to the logger's Mutex.
pub fn get() -> &'static IntMutex<Logger> {
	&LOGGER
}

/// Kernel logger, used to print/store kernel logs.
pub struct Logger {
	/// Tells whether the logger is silent.
	silent: bool,

	/// The buffer storing the kernel logs.
	buff: [u8; LOGS_SIZE],
	/// The buffer's reading head.
	read_head: usize,
	/// The buffer's writing head.
	write_head: usize,
}

impl Logger {
	/// Creates a new instance.
	pub const fn new() -> Self {
		Logger {
			silent: false,

			buff: [0; LOGS_SIZE],
			read_head: 0,
			write_head: 0,
		}
	}

	/// Tells whether the logger is silent.
	#[inline]
	pub fn is_silent(&self) -> bool {
		self.silent
	}

	/// Sets the logger as silent or not.
	#[inline]
	pub fn set_silent(&mut self, silent: bool) {
		self.silent = silent;
	}

	/// Returns the number of bytes used in the buffer.
	pub fn get_size(&self) -> usize {
		self.buff.len() - self.available_space()
	}

	/// Returns the number of available bytes in the buffer.
	fn available_space(&self) -> usize {
		if self.write_head == self.read_head {
			self.buff.len()
		} else if self.write_head > self.read_head {
			self.buff.len() - (self.write_head - self.read_head)
		} else {
			self.read_head - self.write_head - 1
		}
	}

	/// Returns a reference to a slice containing the logs stored into the
	/// loggers's buffer.
	pub fn get_content(&self) -> &[u8] {
		&self.buff
	}

	/// Pushes the given string onto the kernel logs buffer.
	pub fn push(&mut self, s: &[u8]) {
		if self.available_space() < s.len() {
			self.pop(s.len() - self.available_space());
		}

		let len = min(self.available_space(), s.len());
		let end = (self.write_head + len) % self.buff.len();
		if end < self.write_head {
			self.buff[self.write_head..].copy_from_slice(&s[0..(len - end)]);
			self.buff[0..end].copy_from_slice(&s[(len - end)..]);
		} else {
			self.buff[self.write_head..end].copy_from_slice(&s[0..len]);
		}
		self.write_head = end;
	}

	/// Pops at least `n` characters from the buffer. If the popping `n`
	/// characters result in cutting a line, the function shall pop the full
	/// line.
	fn pop(&mut self, n: usize) {
		let read_new = (self.read_head + n) % self.buff.len();
		if read_new >= self.write_head && read_new < self.read_head {
			self.read_head = self.write_head;
			return;
		}

		let mut i = 0;
		while i < self.buff.len() {
			let off = (read_new + i) % self.buff.len();
			if off >= self.write_head || self.buff[off] == b'\n' {
				break;
			}

			i += 1;
		}

		self.read_head = (read_new + i) % self.buff.len();
	}
}

impl Write for Logger {
	fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
		if !self.is_silent() {
			tty::get(None).unwrap().lock().write(s.as_bytes());
		}
		self.push(s.as_bytes());

		Ok(())
	}
}
