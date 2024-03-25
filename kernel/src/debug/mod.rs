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

use crate::{elf, memory};
use core::{ffi::c_void, ptr::null_mut};
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
pub unsafe fn get_callstack(mut frame: *mut usize, stack: &mut [*mut c_void]) {
	stack.fill(null_mut::<c_void>());
	for f in stack.iter_mut() {
		if frame.is_null() {
			break;
		}
		let pc = (*frame.add(1)) as *mut c_void;
		if pc < memory::PROCESS_END {
			break;
		}
		*f = pc;
		frame = *frame as *mut usize;
	}
}

/// Prints a callstack, including symbols' names and addresses.
///
/// `stack` is the callstack to print.
///
/// If the callstack is empty, the function just prints `Empty`.
pub fn print_callstack(stack: &[*mut c_void]) {
	if !matches!(stack.first(), Some(p) if !p.is_null()) {
		crate::println!("Empty");
		return;
	}
	for (i, pc) in stack.iter().enumerate() {
		if pc.is_null() {
			break;
		}
		let name = elf::kernel::get_function_name(*pc).unwrap_or(b"???");
		crate::println!("{i}: {pc:p} -> {}", DisplayableStr(name));
	}
}
