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

//! I/O vectors allow passing several buffers at once to a system call.
//!
//! This feature allows reducing the overhead linked to context switches.

use core::ffi::c_void;

/// An entry of an IO vector used for sparse buffers IO.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct IOVec {
	/// Starting address.
	pub iov_base: *mut c_void,
	/// Number of bytes to transfer.
	pub iov_len: usize,
}

// TODO add a function to turn into an entry into a SyscallSlice?
