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

//! The `user_desc` structure, is used in userspace to specify the value for either a local or
//! global descriptor.

use crate::gdt;
use core::{ffi::c_void, fmt};

/// The size of the `user_desc` structure in bytes.
pub const USER_DESC_SIZE: usize = 16;

/// The `user_desc` structure.
#[repr(transparent)]
pub struct UserDesc {
	val: [i8; USER_DESC_SIZE],
}

impl UserDesc {
	/// Returns the entry number.
	#[inline(always)]
	pub fn get_entry_number(&self) -> i32 {
		unsafe {
			// Safe because the structure is large enough
			*(&self.val[0] as *const _ as *const i32)
		}
	}

	/// Sets the entry number.
	pub fn set_entry_number(&mut self, number: i32) {
		// Safe because the structure is large enough
		unsafe {
			let ptr = &mut self.val[0] as *mut _ as *mut i32;
			*ptr = number;
		}
	}

	/// Returns the base address.
	#[inline(always)]
	pub fn get_base_addr(&self) -> *const c_void {
		// Safe because the structure is large enough
		(unsafe { *(&self.val[4] as *const _ as *const i32) }) as _
	}

	/// Returns the limit.
	#[inline(always)]
	pub fn get_limit(&self) -> i32 {
		// Safe because the structure is large enough
		(unsafe { *(&self.val[8] as *const _ as *const i32) }) as _
	}

	/// Tells whether the segment is 32 bits.
	#[inline(always)]
	pub fn is_32bits(&self) -> bool {
		(self.val[12] & 0b1) != 0
	}

	/// Tells whether the segment is writable.
	#[inline(always)]
	pub fn is_read_exec_only(&self) -> bool {
		(self.val[12] & 0b1000) != 0
	}

	/// Tells whether the segment's limit is in number of pages.
	#[inline(always)]
	pub fn is_limit_in_pages(&self) -> bool {
		(self.val[12] & 0b10000) != 0
	}

	/// Tells whether the segment is present.
	#[inline(always)]
	pub fn is_present(&self) -> bool {
		(self.val[12] & 0b100000) == 0
	}

	/// Tells whether the segment is usable.
	#[inline(always)]
	pub fn is_usable(&self) -> bool {
		(self.val[12] & 0b1000000) != 0
	}

	/// Converts the current descriptor to a GDT entry.
	pub fn to_descriptor(&self) -> gdt::Entry {
		let mut access_byte = 0b01110010;
		if self.is_present() && self.is_usable() {
			access_byte |= 1 << 7;
		}
		if self.is_read_exec_only() {
			access_byte |= 1 << 3;
		}

		let mut flags = 0b0000;
		if self.is_32bits() {
			flags |= 1 << 2;
		}
		if self.is_limit_in_pages() {
			flags |= 1 << 3;
		}

		let mut entry = gdt::Entry::default();
		entry.set_base(self.get_base_addr() as _);
		entry.set_limit(self.get_limit() as _);
		entry.set_access_byte(access_byte);
		entry.set_flags(flags);
		entry
	}
}

impl fmt::Debug for UserDesc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "entry_number: {}, ", self.get_entry_number())?;
		write!(f, "base_addr: {:p}, ", self.get_base_addr())?;
		write!(f, "limit: {:x}, ", self.get_limit())?;
		write!(f, "seg_32bit: {}, ", self.is_32bits())?;
		write!(f, "read_exec_only: {}, ", self.is_read_exec_only())?;
		write!(f, "limit_in_pages: {}, ", self.is_limit_in_pages())?;
		write!(f, "seg_not_present: {}, ", !self.is_present())?;
		write!(f, "useable: {}", self.is_usable())
	}
}
