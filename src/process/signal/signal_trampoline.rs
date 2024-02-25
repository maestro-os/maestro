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

//! A signal handler trampoline is the function that handles returning from a signal handler.
//!
//! The trampoline is using the same stack as the normal process execution.
//!
//! However, the **System V ABI** defines a region of the stack located after the
//! allocated portion which is called the **redzone**. This region must not be
//! clobbered, thus the kernel adds an offset on the stack corresponding to the
//! size of the redzone.
//!
//! When the signal handler returns, the process returns directly to execution.

use core::arch::asm;
use core::ffi::c_void;
use core::mem::transmute;

/// The signal handler trampoline.
///
/// The process resumes to this function when it received a signal.
/// Thus, this code is executed in userspace.
///
/// When the process finished handling the signal, it calls the `sigreturn`
/// system call in order to tell the kernel to resume normal execution.
///
/// Arguments:
/// - `handler` is a pointer to the handler function for the signal.
/// - `sig` is the signal number.
#[no_mangle]
pub extern "C" fn signal_trampoline(handler: *const c_void, sig: i32) -> ! {
	// Calling the signal handler
	unsafe {
		let handler = transmute::<*const c_void, unsafe extern "C" fn(i32)>(handler);
		handler(sig);
	}

	// Calling `sigreturn` to end signal handling.
	unsafe {
		asm!("mov eax, 0x077\nint 0x80");
	}

	unreachable!();
}
