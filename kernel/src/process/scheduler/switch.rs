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

use crate::process::{Process, TLS_ENTRIES_COUNT};
use core::{arch::global_asm, mem::offset_of};

/// Switches context from `prev` to `next`.
///
/// After returning, the execution will continue on `next`.
#[inline]
pub fn switch(prev: &mut Process, next: &mut Process) {
	unsafe {
		switch_asm(prev, next);
	}
}

extern "C" {
	fn switch_asm(prev: &mut Process, next: &mut Process);
}

// TODO 32 bit

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

/// Jumped to from [`switch_asm`], finishing the switch.
#[no_mangle]
extern "C" fn switch_finish(_prev: &mut Process, next: &mut Process) {
	// Bind the memory space
	next.get_mem_space().unwrap().lock().bind();
	// Update the TSS for the process
	next.update_tss();
	// Update TLS entries in the GDT
	for i in 0..TLS_ENTRIES_COUNT {
		next.update_tls(i);
	}
	// TODO switch FPU
}
