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

//! Implementation of registers handling for each architecture.

use crate::gdt;
use core::{arch::asm, fmt};
use utils::errno::EResult;

/// The default value of the eflags register.
const DEFAULT_EFLAGS: u32 = 0x1202;
/// The default value of the FCW.
const DEFAULT_FCW: u32 = 0b1100111111;
/// The default value of the MXCSR.
const DEFAULT_MXCSR: u32 = 0b1111111000000;

extern "C" {
	/// This function switches to a userspace context.
	///
	/// Arguments:
	/// - `regs` is the structure of registers to restore to resume the context.
	/// - `data_selector` is the user data segment selector.
	/// - `code_selector` is the user code segment selector.
	fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
	/// This function switches to a kernelspace context.
	///
	/// `regs` is the structure of registers to restore to resume the context.
	fn context_switch_kernel(regs: &Regs) -> !;
}

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

/// Structure representing the list of registers for a context.
///
/// The content of this structure depends on the architecture for which the kernel is compiled.
#[derive(Clone, Debug)]
#[repr(C, packed)]
#[cfg(target_arch = "x86")]
pub struct Regs {
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

impl Regs {
	/// Sets the return value of a system call.
	pub fn set_syscall_return(&mut self, value: EResult<i32>) {
		let retval = match value {
			Ok(val) => val as _,
			Err(e) => (-e.as_int()) as _,
		};

		self.eax = retval;
	}

	/// Switches to the associated register context.
	/// `user` tells whether the function switchs to userspace.
	///
	/// # Safety
	///
	/// Invalid register values shall result in an undefined behaviour.
	pub unsafe fn switch(&self, user: bool) -> ! {
		let eip = self.eip;
		debug_assert_ne!(eip, 0);
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
			ebp: 0x0,
			esp: 0x0,
			eip: 0x0,
			eflags: DEFAULT_EFLAGS,
			eax: 0x0,
			ebx: 0x0,
			ecx: 0x0,
			edx: 0x0,
			esi: 0x0,
			edi: 0x0,

			gs: 0x0,
			fs: 0x0,

			fxstate: [0; 512],
		};

		// Setting the default FPU control word
		s.fxstate[0] = (DEFAULT_FCW & 0xff) as _;
		s.fxstate[1] = ((DEFAULT_FCW >> 8) & 0xff) as _;

		// Setting the default MXCSR
		s.fxstate[24] = (DEFAULT_MXCSR & 0xff) as _;
		s.fxstate[25] = ((DEFAULT_MXCSR >> 8) & 0xff) as _;
		s.fxstate[26] = ((DEFAULT_MXCSR >> 16) & 0xff) as _;
		s.fxstate[27] = ((DEFAULT_MXCSR >> 24) & 0xff) as _;

		s
	}
}

//#[cfg(config_general_arch = "x86")]
impl fmt::Display for Regs {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"ebp: {:08x} esp: {:08x} eip: {:08x} eax: {:08x} ebx: {:08x}
ecx: {:08x} edx: {:08x} esi: {:08x} edi: {:08x} eflags: {:08x}
gs: {:02x} fs: {:02x}",
			self.ebp as usize,
			self.esp as usize,
			self.eip as usize,
			self.eax as usize,
			self.ebx as usize,
			self.ecx as usize,
			self.edx as usize,
			self.esi as usize,
			self.edi as usize,
			self.eflags as usize,
			self.gs as usize,
			self.fs as usize
		)
	}
}
