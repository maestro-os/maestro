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

use crate::{debug, device::serial, memory::VirtAddr, register_get};
use core::ptr;

/// The operation being sampled.
#[repr(u8)]
pub enum SampleOp {
	Alloc = 0,
	Realloc = 1,
	Free = 2,
}

/// Writes a memory tracing sample to the **COM2** serial port.
///
/// Arguments:
/// - `allocator` is the name of the allocator.
/// - `op` is the operation number.
/// - `ptr` is the affected pointer.
/// - `size` is the new size of the allocation. The unit is dependent on the allocator.
pub fn sample(allocator: &str, op: SampleOp, addr: usize, size: usize) {
	// Dump callstack
	#[cfg(target_arch = "x86")]
	let frame = register_get!("ebp");
	#[cfg(target_arch = "x86_64")]
	let frame = register_get!("rbp");
	let frame = ptr::with_exposed_provenance::<usize>(frame);
	let mut callstack: [VirtAddr; 64] = [VirtAddr::default(); 64];
	unsafe {
		debug::get_callstack(frame, &mut callstack);
	}
	// COM2
	let mut serial = serial::PORTS[1].lock();
	// Write name of allocator
	serial.write(&[allocator.len() as u8]);
	serial.write(allocator.as_bytes());
	// Write op
	serial.write(&[op as u8]);
	// Write ptr and size
	serial.write(&(addr as u64).to_le_bytes());
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
		serial.write(&(f.0 as u64).to_le_bytes());
	}
}
