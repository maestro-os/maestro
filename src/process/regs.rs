//! This module implements the Regs structure, allowing to save an execution state and to restore
//! it.

use core::ffi::c_void;
use core::fmt;
use crate::gdt;

/// The default value of the eflags register.
const DEFAULT_EFLAGS: u32 = 0x1202;
/// The default value of the FPCW.
const DEFAULT_FPCW: u32 = 0b1100111111;
/// The default value of the MXCSR.
const DEFAULT_MXCSR: u32 = 0b1111111000000;

extern "C" {
	/// This function switches to a userspace context.
	/// `regs` is the structure of registers to restore to resume the context.
	/// `data_selector` is the user data segment selector.
	/// `code_selector` is the user code segment selector.
	pub fn context_switch(regs: &Regs, data_selector: u16, code_selector: u16) -> !;
	/// This function switches to a kernelspace context.
	/// `regs` is the structure of registers to restore to resume the context.
	pub fn context_switch_kernel(regs: &Regs) -> !;
}

/// Structure representing the list of registers for a context. The content of this structure
/// depends on the architecture for which the kernel is compiled.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
//#[cfg(config_general_arch = "x86")]
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

	// TODO Add floating-point registers

	pub fpcw: u32,
	pub mxcsr: u32,
}

impl Regs {
	/// Switches to the associated register context.
	/// `user` tells whether the function switchs to userspace.
	/// The function is unsafe because invalid register values shall result in an undefined
	/// behaviour.
	pub unsafe fn switch(&self, user: bool) -> ! {
		if user {
			let user_data_selector = gdt::USER_DATA_OFFSET | 3;
			let user_code_selector = gdt::USER_CODE_OFFSET | 3;

			context_switch(self, user_data_selector as _, user_code_selector as _);
		} else {
			context_switch_kernel(self);
		}
	}
}

impl Default for Regs {
	fn default() -> Self {
		Self {
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

			fpcw: DEFAULT_FPCW,
			mxcsr: DEFAULT_MXCSR,
		}
	}
}

impl fmt::Display for Regs {
	//#[cfg(config_general_arch = "x86")]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ebp: {:p} esp: {:p} eip: {:p} eflags: {:p} eax: {:p}\n
ebx: {:p} ecx: {:p} edx: {:p} esi: {:p} edi: {:p}\n",
			self.ebp as *const c_void,
			self.esp as *const c_void,
			self.eip as *const c_void,
			self.eflags as *const c_void,
			self.eax as *const c_void,
			self.ebx as *const c_void,
			self.ecx as *const c_void,
			self.edx as *const c_void,
			self.esi as *const c_void,
			self.edi as *const c_void)

		// TODO Print all registers
	}
}
