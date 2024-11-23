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

//! Context switching utilities.

use crate::{
	arch::x86::idt::IntFrame,
	process::{Process, TLS_ENTRIES_COUNT},
};
use core::{
	arch::{asm, global_asm},
	mem::offset_of,
};

/// Jumps to a context with the given `frame`.
pub fn init(frame: &IntFrame) -> ! {
	#[cfg(target_arch = "x86")]
	unsafe {
		asm!(
			r"mov esp, {}
			LOAD_REGS
			add esp, 8
			iretd",
			in(reg) frame,
			options(noreturn)
		)
	}
	#[cfg(target_arch = "x86_64")]
	unsafe {
		asm!(
			r"mov esp, {}
			LOAD_REGS
			add esp, 8
			iretd",
			in(reg) frame,
			options(noreturn)
		)
	}
}

/// Switches context from `prev` to `next`.
///
/// After returning, the execution will continue on `next`.
///
/// # Safety
///
/// The pointers must point to valid processes.
pub unsafe fn switch(prev: *const Process, next: *const Process) {
	switch_asm(prev, next);
}

extern "C" {
	#[allow(improper_ctypes)]
	fn switch_asm(prev: *const Process, next: *const Process);
}

#[cfg(target_arch = "x86")]
global_asm!(r"
switch_asm:
	push ebp
	push ebx

    # Swap contexts
    mov [edi + {off}], esp
    mov esp, [esi + {off}]

	push ebx
	push ebp

	jmp switch_finish
", off = const offset_of!(Process, kernel_sp));

#[cfg(target_arch = "x86_64")]
global_asm!(r"
switch_asm:
	push rbp
	push rbx
	push r12
	push r13
	push r14
	push r15

    # Swap contexts
    mov [rdi + {off}], rsp
    mov rsp, [rsi + {off}]

	push r15
	push r14
	push r13
	push r12
	push rbx
	push rbp

	jmp switch_finish
", off = const offset_of!(Process, kernel_sp));

/// Jumped to from [`switch`], finishing the switch.
#[no_mangle]
extern "C" fn switch_finish(_prev: &mut Process, next: &mut Process) {
	// Bind the memory space
	next.mem_space.as_ref().unwrap().lock().bind();
	// Update the TSS for the process
	next.update_tss();
	// Update TLS entries in the GDT
	for i in 0..TLS_ENTRIES_COUNT {
		next.update_tls(i);
	}
	// TODO switch FPU
}
