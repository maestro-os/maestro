//! This module implements the Regs structure, allowing to save an execution state and to restore
//! it.

use core::ffi::c_void;
use core::fmt;
use crate::gdt;

/// The default value of the eflags register.
const DEFAULT_EFLAGS: u32 = 0x1202;
/// The default value of the FCW.
const DEFAULT_FCW: u32 = 0b1100111111;
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

// TODO Ensure the buffer is on a 32 bits address (required by fxsave and fxrstor)
/// Wrapper allowing to align the fxstate buffer.
#[repr(align(16))]
struct FXStateWrapper([u8; 512]);

/// Saves the current x87 FPU, MMX and SSE state to the given buffer.
#[no_mangle]
pub extern "C" fn save_fxstate(fxstate: &mut [u8; 512]) {
	let mut buff = FXStateWrapper([0; 512]);
	unsafe {
		core::arch::asm!("fxsave [eax]", in("eax") &mut buff);
	}

	fxstate.copy_from_slice(&buff.0);
}

/// Restores the x87 FPU, MMX and SSE state from the given buffer.
#[no_mangle]
pub extern "C" fn restore_fxstate(fxstate: &[u8; 512]) {
	let mut buff = FXStateWrapper([0; 512]);
	buff.0.copy_from_slice(fxstate);

	unsafe {
		core::arch::asm!("fxrstor [eax]", in("eax") &buff);
	}
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

	pub gs: u32,
	pub fs: u32,

	/// x87 FPU, MMX and SSE state.
	pub fxstate: [u8; 512],
}

impl Regs {
	/// Switches to the associated register context.
	/// `user` tells whether the function switchs to userspace.
	/// The function is unsafe because invalid register values shall result in an undefined
	/// behaviour.
	pub unsafe fn switch(&self, user: bool) -> ! {
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

impl fmt::Display for Regs {
	//#[cfg(config_general_arch = "x86")]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let fs = self.fs;
		let gs = self.gs;

		write!(f, "ebp: {:p} esp: {:p} eip: {:p} eflags: {:p} eax: {:p}\n
ebx: {:p} ecx: {:p} edx: {:p} esi: {:p} edi: {:p}\n
gs: {:x} fs: {:x}\n",
			self.ebp as *const c_void,
			self.esp as *const c_void,
			self.eip as *const c_void,
			self.eflags as *const c_void,
			self.eax as *const c_void,
			self.ebx as *const c_void,
			self.ecx as *const c_void,
			self.edx as *const c_void,
			self.esi as *const c_void,
			self.edi as *const c_void,
			gs,
			fs)
	}
}
