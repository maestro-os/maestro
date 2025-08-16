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

//! Debugging tools for the kernel.

use crate::{elf, memory, memory::VirtAddr, println};
use core::ptr;
use utils::DisplayableStr;

/// Fills the slice `stack` with the callstack starting at `frame`.
///
/// The first element is the last called function and the last element is the first called
/// function.
///
/// When the stack ends, the function fills the rest of the slice with `None`.
///
/// # Safety
///
/// The caller must ensure the `frame` parameter points ta a valid stack frame.
pub unsafe fn get_callstack(mut frame: *const usize, stack: &mut [VirtAddr]) {
	stack.fill(VirtAddr::default());
	for f in stack.iter_mut() {
		if frame.is_null() {
			break;
		}
		let pc = ptr::read_unaligned(frame.add(1) as _);
		if pc < memory::PROCESS_END {
			break;
		}
		*f = pc;
		frame = ptr::read_unaligned(frame as *const *const usize);
	}
}

/// Prints a callstack, including symbols' names and addresses.
///
/// `stack` is the callstack to print.
///
/// If the callstack is empty, the function just prints `Empty`.
pub fn print_callstack(stack: &[VirtAddr]) {
	if !matches!(stack.first(), Some(p) if !p.is_null()) {
		println!();
		return;
	}
	for pc in stack.iter() {
		if pc.is_null() {
			break;
		}
		let name = elf::kernel::get_function_name(*pc).unwrap_or(b"???");
		println!(" <{pc:?}>: {}", DisplayableStr(name));
	}
}

/// Utilities to manipulate QEMU.
#[cfg(config_debug_qemu)]
pub mod qemu {
	use crate::{arch::x86::io::outl, power};

	/// The port used to trigger QEMU emulator exit with the given exit code.
	const EXIT_PORT: u16 = 0xf4;

	/// QEMU exit code for success.
	pub const SUCCESS: u32 = 0x10;
	/// QEMU exit code for failure.
	pub const FAILURE: u32 = 0x11;

	/// Exits QEMU with the given status.
	pub fn exit(status: u32) {
		unsafe {
			outl(EXIT_PORT, status);
		}
		// halt in case exiting did not succeed for some reason
		power::halt();
	}
}
