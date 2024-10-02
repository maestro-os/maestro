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

//! Registers state save and restore.

use crate::{gdt, memory::VirtAddr};
use core::fmt;
use utils::errno::EResult;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86 {
	use super::Regs;
	use core::arch::{asm, global_asm};

	/// The default value of the flags register.
	pub const DEFAULT_FLAGS: usize = 0x202;
	/// The default value of the FCW.
	pub const DEFAULT_FCW: u32 = 0b1100111111;
	/// The default value of the MXCSR.
	pub const DEFAULT_MXCSR: u32 = 0b1111111000000;

	/// FXstate buffer.
	#[repr(align(16))]
	struct FXState([u8; 512]);

	/// Saves the current x87 FPU, MMX and SSE state to the given buffer.
	#[no_mangle]
	pub extern "C" fn save_fxstate(fxstate: &mut [u8; 512]) {
		let mut buf = FXState([0; 512]);
		unsafe {
			asm!("fxsave [{}]", in(reg) buf.0.as_mut_ptr());
		}
		// TODO avoid copy (slow). `fxstate` itself should be aligned
		fxstate.copy_from_slice(&buf.0);
	}

	/// Restores the x87 FPU, MMX and SSE state from the given buffer.
	#[no_mangle]
	pub extern "C" fn restore_fxstate(fxstate: &[u8; 512]) {
		let mut buf = FXState([0; 512]);
		// TODO avoid copy (slow). `fxstate` itself should be aligned
		buf.0.copy_from_slice(fxstate);
		unsafe {
			asm!("fxrstor [{}]", in(reg) buf.0.as_ptr());
		}
	}

	extern "C" {
		/// Switches to an userspace context.
		///
		/// Arguments:
		/// - `regs` is the structure of registers to restore to resume the context.
		/// - `data_selector` is the user data segment selector.
		/// - `code_selector` is the user code segment selector.
		pub(super) fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
		/// Switches to a kernelspace context.
		///
		/// `regs` is the structure of registers to restore to resume the context.
		pub(super) fn context_switch_kernel(regs: &Regs) -> !;
	}

	global_asm!(
		r"
.section .text

.global context_switch
.global context_switch_kernel

.type context_switch, @function
.type context_switch_kernel, @function

context_switch:
	# Set segment registers
	mov eax, [esp + 8]
	mov ds, ax
	mov es, ax

	# Restore the fx state
	mov eax, [esp + 4]
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	# Set registers, except eax
	mov eax, [esp + 4]
	mov ebp, [eax]
	mov ebx, [eax + 20]
	mov ecx, [eax + 24]
	mov edx, [eax + 28]
	mov esi, [eax + 32]
	mov edi, [eax + 36]
	mov gs, [eax + 40]
	mov fs, [eax + 44]

	# Place iret data on the stack
	push [esp + 8] # data segment selector
	push [eax + 4] # esp
	push [eax + 12] # eflags
	push [esp + 24] # code segment selector
	push [eax + 8] # eip

	# Set eax
	mov eax, [eax + 16]

	iret

context_switch_kernel:
	# Restore the fx state
	mov eax, [esp + 4]
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	mov eax, [esp + 4]

	# Set eflags without the interrupt flag
	mov ebx, [eax + 12]
	mov ecx, 512
	not ecx
	and ebx, ecx
	push ebx
	popf

	# Set registers
	mov ebp, [eax]
	mov esp, [eax + 4]
	push [eax + 8] # eip
	mov [eax + 20], ebx
	mov [eax + 24], ecx
	mov [eax + 28], edx
	mov [eax + 32], esi
	mov [eax + 36], edi
	mov [eax + 40], gs
	mov [eax + 44], fs
	mov [eax + 16], eax

	# Set the interrupt flag and jumping to kernel code execution
	# (Note: These two instructions, if placed in this order are atomic on x86, meaning that an interrupt cannot happen in between)
	sti
	ret"
	);
}

/// The register state of an execution context.
///
/// The contents of this structure is architecture-dependent.
#[derive(Clone)]
#[repr(C)]
#[allow(missing_docs)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct Regs {
	pub ebp: usize,
	pub esp: usize,
	pub eip: usize,
	pub eflags: usize,
	pub eax: usize,
	pub ebx: usize,
	pub ecx: usize,
	pub edx: usize,
	pub esi: usize,
	pub edi: usize,

	pub gs: usize,
	pub fs: usize,

	/// x87 FPU, MMX and SSE state.
	pub fxstate: [u8; 512],
}

impl Regs {
	/// Returns the ID of the system call being executed.
	#[inline]
	pub const fn get_syscall_id(&self) -> usize {
		self.eax
	}

	/// Returns the value of the `n`th argument of the syscall being executed.
	///
	/// If `n` exceeds the number of arguments for the current architecture, the function returns
	/// `0`.
	#[inline]
	pub const fn get_syscall_arg(&self, n: u8) -> usize {
		match n {
			0 => self.ebx,
			1 => self.ecx,
			2 => self.edx,
			3 => self.esi,
			4 => self.edi,
			5 => self.ebp,
			_ => 0,
		}
	}

	/// Sets the return value of a system call.
	pub fn set_syscall_return(&mut self, value: EResult<usize>) {
		self.eax = value.unwrap_or_else(|e| (-e.as_int()) as _);
	}

	/// Switches to the associated register context.
	/// `user` tells whether the function switches to userspace.
	///
	/// # Safety
	///
	/// Invalid register values shall result in an undefined behaviour.
	pub unsafe fn switch(&self, user: bool) -> ! {
		let pc = self.eip;
		debug_assert_ne!(pc, 0);
		if user {
			let user_data_selector = gdt::USER_DS | 3;
			let user_code_selector = gdt::USER_CS | 3;
			x86::context_switch(self, user_data_selector as _, user_code_selector as _);
		} else {
			x86::context_switch_kernel(self);
		}
	}
}

impl Default for Regs {
	fn default() -> Self {
		let mut s = Self {
			ebp: 0,
			esp: 0,
			eip: 0,
			eflags: x86::DEFAULT_FLAGS,
			eax: 0,
			ebx: 0,
			ecx: 0,
			edx: 0,
			esi: 0,
			edi: 0,

			gs: 0,
			fs: 0,

			fxstate: [0; 512],
		};
		// Set the default FPU control word
		s.fxstate[0] = (x86::DEFAULT_FCW & 0xff) as _;
		s.fxstate[1] = ((x86::DEFAULT_FCW >> 8) & 0xff) as _;
		// Set the default MXCSR
		s.fxstate[24] = (x86::DEFAULT_MXCSR & 0xff) as _;
		s.fxstate[25] = ((x86::DEFAULT_MXCSR >> 8) & 0xff) as _;
		s.fxstate[26] = ((x86::DEFAULT_MXCSR >> 16) & 0xff) as _;
		s.fxstate[27] = ((x86::DEFAULT_MXCSR >> 24) & 0xff) as _;
		s
	}
}

impl fmt::Debug for Regs {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// use `VirtAddr` to avoid duplicate code
		f.debug_struct("Regs")
			.field("ebp", &VirtAddr(self.ebp))
			.field("esp", &VirtAddr(self.esp))
			.field("eip", &VirtAddr(self.eip))
			.field("eflags", &VirtAddr(self.eflags))
			.field("eax", &VirtAddr(self.eax))
			.field("ebx", &VirtAddr(self.ebx))
			.field("ecx", &VirtAddr(self.ecx))
			.field("edx", &VirtAddr(self.edx))
			.field("esi", &VirtAddr(self.esi))
			.field("edi", &VirtAddr(self.edi))
			.field("gs", &VirtAddr(self.gs))
			.field("fs", &VirtAddr(self.fs))
			.finish()
	}
}
