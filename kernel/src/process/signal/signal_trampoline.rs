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

//! A signal trampoline calls a signal handler when a signal occurs, and handling restoring the
//! original context after the handler returns.
//!
//! The trampoline executing in userspace.
//!
//! The trampoline takes as argument:
//! - `handler`: a pointer to the handler function for the signal
//! - `sig`: the signal number
//! - `ctx`: the context to restore after the handler finishes
//!
//! Restoring the original context is done by calling [`crate::syscall::sigreturn::sigreturn`].

use crate::{
	process::signal::ucontext::{UContext32, UContext64},
	syscall::SIGRETURN_ID,
};
use core::arch::asm;

#[link_section = ".user"]
pub unsafe extern "C" fn trampoline32(
	handler: unsafe extern "C" fn(i32),
	sig: usize,
	ctx: &mut UContext32,
) -> ! {
	handler(sig as _);
	// Call `sigreturn`
	asm!(
		"mov esp, {}",
		"int 0x80",
		"ud2",
		in(reg) ctx.uc_stack,
		in("eax") SIGRETURN_ID,
		options(noreturn)
	);
}

#[cfg(target_arch = "x86_64")]
#[link_section = ".user"]
pub unsafe extern "C" fn trampoline64(
	handler: unsafe extern "C" fn(i32),
	sig: usize,
	ctx: &mut UContext64,
) -> ! {
	handler(sig as _);
	// Call `sigreturn`
	asm!(
		"mov rsp, {}",
		"sysenter",
		"ud2",
		in(reg) ctx.uc_stack,
		in("rax") SIGRETURN_ID,
		options(noreturn)
	);
}
