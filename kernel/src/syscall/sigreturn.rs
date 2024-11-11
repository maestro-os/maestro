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

//! The `sigreturn` system call is used whenever the process finished executing
//! a signal handler.
//!
//! The system call restores the previous state of the process
//! to allow normal execution.

use crate::{
	arch::x86::idt::IntFrame,
	process::{
		mem_space::copy::SyscallPtr,
		signal::{ucontext, Signal},
		Process,
	},
	syscall::FromSyscallArg,
};
use core::{mem::size_of, ptr};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	lock::{IntMutex, IntMutexGuard},
};

pub fn sigreturn(frame: &mut IntFrame) -> EResult<usize> {
	let proc = Process::current();
	// Retrieve and restore previous state
	let ctx_ptr = frame.get_stack_address();
	if frame.is_32bit() {
		let ctx = SyscallPtr::<ucontext::UContext32>::from_syscall_arg(ctx_ptr);
		let ctx = ctx.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		ctx.restore_regs(&proc, frame);
	} else {
		#[cfg(target_arch = "x86_64")]
		{
			let ctx = SyscallPtr::<ucontext::UContext64>::from_syscall_arg(ctx_ptr);
			let ctx = ctx.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			ctx.restore_regs(&proc, frame);
		}
	}
	// Do not touch register
	Ok(frame.get_syscall_id())
}
