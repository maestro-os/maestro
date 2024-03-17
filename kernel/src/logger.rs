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

//! Kernel logging
//!
//! If the logger is set as silent, logs will not show up on screen, but will be kept in memory
//! anyways.

use crate::tty;
use core::{
	cmp::{min, Ordering},
	fmt,
	fmt::Write,
};
use utils::lock::IntMutex;

/// The size of the kernel logs buffer in bytes.
const LOGS_SIZE: usize = 1048576;

/// The kernel's logger.
pub static LOGGER: IntMutex<Logger> = IntMutex::new(Logger::new());

/// Kernel logger, used to print/store kernel logs.
///
/// Internally, the logger uses a ring buffer for storage.
pub struct Logger {
	/// Tells whether the logger is silent.
	pub silent: bool,

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

	/// Returns the number of bytes used in the buffer.
	pub fn get_size(&self) -> usize {
		self.buff.len() - self.available_space()
	}

	/// Returns the number of available bytes in the buffer.
	fn available_space(&self) -> usize {
		match self.write_head.cmp(&self.read_head) {
			Ordering::Equal => self.buff.len(),
			Ordering::Greater => self.buff.len() - (self.write_head - self.read_head),
			Ordering::Less => self.read_head - self.write_head - 1,
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
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.push(s.as_bytes());
		if !self.silent {
			let Some(tty) = tty::get(None) else {
				return Ok(());
			};
			tty.lock().write(s.as_bytes());
		}
		Ok(())
	}
}
