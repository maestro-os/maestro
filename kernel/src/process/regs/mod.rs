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

use crate::memory::VirtAddr;
use core::fmt;
use utils::errno::EResult;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86 {
	use super::Regs32;
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
		/// Switches to a 32 bit userspace context.
		///
		/// `regs` is the set of registers to restore to resume the context.
		pub(super) fn context_switch32(regs: &Regs32) -> !;
		/// Switches to a 64 bit userspace context.
		///
		/// `regs` is the set of registers to restore to resume the context.
		#[cfg(target_arch = "x86_64")]
		pub(super) fn context_switch64(regs: &super::Regs64) -> !;

		/// Switches to a kernelspace context.
		///
		/// `regs` is the structure of registers to restore to resume the context.
		pub(super) fn context_switch_kernel(regs: &Regs32) -> !;
	}

	#[cfg(target_arch = "x86")]
	global_asm!(
		r"
.section .text

.global context_switch32
.global context_switch_kernel

.type context_switch32, @function
.type context_switch_kernel, @function

context_switch32:
	# Restore the fx state
	mov eax, [esp + 4]
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	# Set segment registers
	mov ax, (32 | 3)
	mov ds, ax
	mov es, ax

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
	push (32 | 3) # data segment selector
	push [eax + 4] # esp
	push [eax + 12] # eflags
	push (24 | 3) # code segment selector
	push [esp + 24] # eip

	# Set eax
	mov eax, [eax + 16]

	iretd

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
	popfd

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

	#[cfg(target_arch = "x86_64")]
	global_asm!(r""); // TODO
}

/// The register state of a 32 bit execution context.
///
/// The contents of this structure is architecture-dependent.
#[derive(Clone)]
#[repr(C)]
#[allow(missing_docs)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct Regs32 {
	pub ebp: u32,
	pub esp: u32,
	pub eip: u32,
	pub eflags: u32,
	pub eax: u32,
	pub ebx: u32,
	pub ecx: u32,
	pub edx: u32,
	pub esi: u32,
	pub edi: u32,

	pub gs: u32,
	pub fs: u32,

	/// x87 FPU, MMX and SSE state.
	pub fxstate: [u8; 512],
}

impl Regs32 {
	/// Returns the ID of the system call being executed.
	#[inline]
	pub const fn get_syscall_id(&self) -> usize {
		self.eax as _
	}

	/// Returns the value of the `n`th argument of the syscall being executed.
	///
	/// If `n` exceeds the number of arguments for the current architecture, the function returns
	/// `0`.
	#[inline]
	pub const fn get_syscall_arg(&self, n: u8) -> usize {
		match n {
			0 => self.ebx as _,
			1 => self.ecx as _,
			2 => self.edx as _,
			3 => self.esi as _,
			4 => self.edi as _,
			5 => self.ebp as _,
			_ => 0,
		}
	}

	/// Sets the return value of a system call.
	pub fn set_syscall_return(&mut self, value: EResult<usize>) {
		self.eax = value.map(|v| v as _).unwrap_or_else(|e| (-e.as_int()) as _);
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
			x86::context_switch32(self);
		} else {
			#[cfg(target_arch = "x86")]
			x86::context_switch_kernel(self);
			#[cfg(target_arch = "x86_64")]
			panic!("attempt to run in 32 bit mode when compiled for x86_64");
		}
	}
}

impl Default for Regs32 {
	fn default() -> Self {
		let mut s = Self {
			ebp: 0,
			esp: 0,
			eip: 0,
			eflags: x86::DEFAULT_FLAGS as _,
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

impl fmt::Debug for Regs32 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// use `VirtAddr` to avoid duplicate code
		f.debug_struct("Regs32")
			.field("ebp", &VirtAddr(self.ebp as _))
			.field("esp", &VirtAddr(self.esp as _))
			.field("eip", &VirtAddr(self.eip as _))
			.field("eflags", &VirtAddr(self.eflags as _))
			.field("eax", &VirtAddr(self.eax as _))
			.field("ebx", &VirtAddr(self.ebx as _))
			.field("ecx", &VirtAddr(self.ecx as _))
			.field("edx", &VirtAddr(self.edx as _))
			.field("esi", &VirtAddr(self.esi as _))
			.field("edi", &VirtAddr(self.edi as _))
			.field("gs", &VirtAddr(self.gs as _))
			.field("fs", &VirtAddr(self.fs as _))
			.finish()
	}
}

/// The register state of a 64 bit execution context.
///
/// The contents of this structure is architecture-dependent.
#[derive(Clone)]
#[repr(C)]
#[allow(missing_docs)]
#[cfg(target_arch = "x86_64")]
pub struct Regs64 {
	pub rbp: u64,
	pub rsp: u64,
	pub rip: u64,
	pub rflags: u64,
	pub rax: u64,
	pub rbx: u64,
	pub rcx: u64,
	pub rdx: u64,
	pub rsi: u64,
	pub rdi: u64,
	// Added by long mode
	pub r8: u64,
	pub r9: u64,
	pub r10: u64,
	pub r11: u64,
	pub r12: u64,
	pub r13: u64,
	pub r14: u64,
	pub r15: u64,

	// TODO check if those are useful here
	pub gs: u32,
	pub fs: u32,

	/// x87 FPU, MMX and SSE state.
	pub fxstate: [u8; 512],
}

#[cfg(target_arch = "x86_64")]
impl fmt::Debug for Regs64 {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// use `VirtAddr` to avoid duplicate code
		f.debug_struct("Regs64")
			.field("rbp", &VirtAddr(self.rbp as _))
			.field("rsp", &VirtAddr(self.rsp as _))
			.field("rip", &VirtAddr(self.rip as _))
			.field("rflags", &VirtAddr(self.rflags as _))
			.field("rax", &VirtAddr(self.rax as _))
			.field("rbx", &VirtAddr(self.rbx as _))
			.field("rcx", &VirtAddr(self.rcx as _))
			.field("rdx", &VirtAddr(self.rdx as _))
			.field("rsi", &VirtAddr(self.rsi as _))
			.field("rdi", &VirtAddr(self.rdi as _))
			.field("r8", &VirtAddr(self.r8 as _))
			.field("r9", &VirtAddr(self.r9 as _))
			.field("r10", &VirtAddr(self.r10 as _))
			.field("r11", &VirtAddr(self.r11 as _))
			.field("r12", &VirtAddr(self.r12 as _))
			.field("r13", &VirtAddr(self.r13 as _))
			.field("r14", &VirtAddr(self.r14 as _))
			.field("r15", &VirtAddr(self.r15 as _))
			.field("gs", &VirtAddr(self.gs as _))
			.field("fs", &VirtAddr(self.fs as _))
			.finish()
	}
}
