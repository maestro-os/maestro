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
//! When the signal handler returns, the process returns directly to execution.

use crate::syscall::SIGRETURN_ID;
use core::arch::asm;

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
#[link_section = ".user"]
pub unsafe extern "C" fn signal_trampoline(handler: unsafe extern "C" fn(i32), sig: i32) -> ! {
	// Call the signal handler
	handler(sig);
	// Call `sigreturn` to end signal handling
	asm!(
		"int 0x80",
		in("eax") SIGRETURN_ID,
		options(noreturn),
	)
}
