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

use core::mem::size_of;
use crate::{
	process::{
		mem_space::copy::SyscallPtr,
		regs::Regs,
		signal::{Signal, UContext},
		Process,
	},
	syscall::FromSyscallArg,
};
use core::ptr;
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	lock::{IntMutex, IntMutexGuard},
};

pub fn sigreturn(regs: &Regs) -> EResult<usize> {
	// Avoid re-enabling interrupts before context switching
	cli();
	// Retrieve the previous state
	let ctx_ptr = regs.esp.0 - size_of::<UContext>();
	let ctx_ptr = SyscallPtr::<UContext>::from_syscall_arg(ctx_ptr);
	let ctx = ctx_ptr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	{
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		// Restores the state of the process before the signal handler
		proc.sigmask = ctx.uc_sigmask;
	}
	// Do not handle the next pending signal here, to prevent signal spamming from completely
	// blocking the process
	unsafe {
		ctx.uc_mcontext.switch(true);
	}
}
