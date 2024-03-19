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

//! Memory usage tracing utility functions.

use crate::{debug, device::serial, register_get};
use core::{ffi::c_void, ptr::null_mut};

/// Writes a memory tracing sample to the **COM2** serial port.
///
/// Arguments:
/// - `allocator` is the name of the allocator.
/// - `op` is the operation number.
/// - `ptr` is the affected pointer.
/// - `size` is the new size of the allocation. The unit is dependent on the allocator.
pub fn sample<T>(allocator: &str, op: u8, ptr: *const T, size: usize) {
	// Dump callstack
	let mut callstack: [*mut c_void; 64] = [null_mut(); 64];
	unsafe {
		let ebp = register_get!("ebp");
		debug::get_callstack(ebp as _, &mut callstack);
	}
	// COM2
	let mut serial = serial::PORTS[1].lock();
	// Write name of allocator
	serial.write(&[allocator.len() as u8]);
	serial.write(allocator.as_bytes());
	// Write op
	serial.write(&[op]);
	// Write ptr and size
	serial.write(&(ptr as u64).to_le_bytes());
	serial.write(&(size as u64).to_le_bytes());
	// Write callstack
	let len = callstack
		.iter()
		.enumerate()
		.find(|(_, p)| p.is_null())
		.map(|(i, _)| i)
		.unwrap_or(callstack.len());
	serial.write(&[len as u8]);
	for f in &callstack[..len] {
		serial.write(&(*f as u64).to_le_bytes());
	}
}
