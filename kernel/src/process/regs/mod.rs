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

use crate::gdt;
use core::{fmt, mem::size_of};
use utils::errno::EResult;

extern "C" {
	/// Switches to an userspace context.
	///
	/// Arguments:
	/// - `regs` is the structure of registers to restore to resume the context.
	/// - `data_selector` is the user data segment selector.
	/// - `code_selector` is the user code segment selector.
	fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
	/// Switches to a kernelspace context.
	///
	/// `regs` is the structure of registers to restore to resume the context.
	fn context_switch_kernel(regs: &Regs) -> !;
}

/// A register's state.
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct Register(pub usize);

impl fmt::Display for Register {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl fmt::Debug for Register {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		const LEN: usize = size_of::<usize>() * 2;
		write!(f, "{:0LEN$x}", self.0)
	}
}

#[cfg(target_arch = "x86")]
mod x86 {
	use core::arch::asm;

	/// The default value of the eflags register.
	pub const DEFAULT_EFLAGS: usize = 0x202;
	/// The default value of the FCW.
	pub const DEFAULT_FCW: u32 = 0b1100111111;
	/// The default value of the MXCSR.
	pub const DEFAULT_MXCSR: u32 = 0b1111111000000;

	/// Wrapper allowing to align the fxstate buffer.
	#[repr(align(16))]
	struct FXStateWrapper([u8; 512]);

	/// Saves the current x87 FPU, MMX and SSE state to the given buffer.
	#[no_mangle]
	pub extern "C" fn save_fxstate(fxstate: &mut [u8; 512]) {
		let mut buff = FXStateWrapper([0; 512]);
		unsafe {
			asm!("fxsave [{}]", in(reg) buff.0.as_mut_ptr());
		}

		// TODO avoid copy (slow). `fxstate` itself should be aligned
		fxstate.copy_from_slice(&buff.0);
	}

	/// Restores the x87 FPU, MMX and SSE state from the given buffer.
	#[no_mangle]
	pub extern "C" fn restore_fxstate(fxstate: &[u8; 512]) {
		let mut buff = FXStateWrapper([0; 512]);
		// TODO avoid copy (slow). `fxstate` itself should be aligned
		buff.0.copy_from_slice(fxstate);

		unsafe {
			asm!("fxrstor [{}]", in(reg) buff.0.as_ptr());
		}
	}
}

/// The register state of an execution context.
///
/// The contents of this structure is architecture-dependent.
#[derive(Clone)]
#[repr(C)]
#[allow(missing_docs)]
#[cfg(target_arch = "x86")]
pub struct Regs {
	pub ebp: Register,
	pub esp: Register,
	pub eip: Register,
	pub eflags: Register,
	pub eax: Register,
	pub ebx: Register,
	pub ecx: Register,
	pub edx: Register,
	pub esi: Register,
	pub edi: Register,

	pub gs: Register,
	pub fs: Register,

	/// x87 FPU, MMX and SSE state.
	pub fxstate: [u8; 512],
}

impl Regs {
	/// Returns the ID of the system call being executed.
	#[inline]
	pub const fn get_syscall_id(&self) -> usize {
		self.eax.0 as _
	}

	/// Returns the value of the `n`th argument of the syscall being executed.
	///
	/// If `n` exceeds the number of arguments for the current architecture, the function returns
	/// `0`.
	#[inline]
	pub const fn get_syscall_arg(&self, n: u8) -> usize {
		match n {
			0 => self.ebx.0 as _,
			1 => self.ecx.0 as _,
			2 => self.edx.0 as _,
			3 => self.esi.0 as _,
			4 => self.edi.0 as _,
			5 => self.ebp.0 as _,
			_ => 0,
		}
	}

	/// Sets the return value of a system call.
	pub fn set_syscall_return(&mut self, value: EResult<usize>) {
		let retval = match value {
			Ok(val) => val as _,
			Err(e) => (-e.as_int()) as _,
		};
		self.eax.0 = retval;
	}

	/// Switches to the associated register context.
	/// `user` tells whether the function switches to userspace.
	///
	/// # Safety
	///
	/// Invalid register values shall result in an undefined behaviour.
	pub unsafe fn switch(&self, user: bool) -> ! {
		let pc = self.eip.0;
		debug_assert_ne!(pc, 0);
		if user {
			let user_data_selector = gdt::USER_DS | 3;
			let user_code_selector = gdt::USER_CS | 3;
			context_switch(self, user_data_selector as _, user_code_selector as _);
		} else {
			context_switch_kernel(self);
		}
	}
}

impl Default for Regs {
	fn default() -> Self {
		let mut s = Self {
			ebp: Register::default(),
			esp: Register::default(),
			eip: Register::default(),
			eflags: Register(x86::DEFAULT_EFLAGS),
			eax: Register::default(),
			ebx: Register::default(),
			ecx: Register::default(),
			edx: Register::default(),
			esi: Register::default(),
			edi: Register::default(),

			gs: Register::default(),
			fs: Register::default(),

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

#[cfg(target_arch = "x86")]
impl fmt::Debug for Regs {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let ebp = self.ebp;
		let esp = self.esp;
		let eip = self.eip;
		let eflags = self.eflags;
		let eax = self.eax;
		let ebx = self.ebx;
		let ecx = self.ecx;
		let edx = self.edx;
		let esi = self.esi;
		let edi = self.edi;
		let gs = self.gs;
		let fs = self.fs;
		f.debug_struct("Regs")
			.field("ebp", &ebp)
			.field("esp", &esp)
			.field("eip", &eip)
			.field("eflags", &eflags)
			.field("eax", &eax)
			.field("ebx", &ebx)
			.field("ecx", &ecx)
			.field("edx", &edx)
			.field("esi", &esi)
			.field("edi", &edi)
			.field("gs", &gs)
			.field("fs", &fs)
			.finish()
	}
}
