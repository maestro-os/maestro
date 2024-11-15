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

//! Userspace signal context structures.

use crate::{
	arch::x86::{gdt, idt::IntFrame},
	process::{signal::SigSet, Process},
};
// TODO restore everything

// ------------------------------
//    32 bit structures

/// General purpose registers (32 bit).
#[repr(usize)]
pub enum GReg32 {
	Gs = 0,
	Fs,
	Es,
	Ds,
	Edi,
	Esi,
	Ebp,
	Esp,
	Ebx,
	Edx,
	Ecx,
	Eax,
	Trapno,
	Err,
	Eip,
	Cs,
	Efl,
	Uesp,
	Ss,
}

/// 32-bit userspace signal context.
#[repr(C)]
#[derive(Debug)]
pub struct UContext32 {
	pub uc_flags: u32,
	pub uc_link: u32, // 32 bit pointer
	pub uc_stack: Stack32,
	pub uc_mcontext: MContext32,
	pub uc_sigmask: SigSet,
	pub __fpregs_mem: FpState32,
	pub __ssp: [u64; 4],
}

impl UContext32 {
	/// Creates a context structure from the current.
	pub fn new(process: &Process, frame: &IntFrame) -> Self {
		Self {
			uc_flags: 0, // TODO
			uc_link: 0,
			// TODO
			uc_stack: Stack32 {
				ss_sp: 0,
				ss_flags: 0,
				ss_size: 0,
			},
			uc_mcontext: MContext32 {
				gregs: [
					frame.gs as _,
					frame.fs as _,
					(gdt::USER_DS | 3) as _,
					(gdt::USER_DS | 3) as _,
					frame.rdi as _,
					frame.rsi as _,
					frame.rbp as _,
					frame.rsp as _,
					frame.rbx as _,
					frame.rdx as _,
					frame.rcx as _,
					frame.rax as _,
					0, // TODO trapno
					0, // TODO err
					frame.rip as _,
					frame.cs as _,
					frame.rflags as _,
					0, // TODO uesp
					frame.ss as _,
				],
				fpregs: 0,  // TODO
				oldmask: 0, // TODO
				cr2: 0,
			},
			uc_sigmask: process.signal.lock().sigmask,
			// TODO
			__fpregs_mem: FpState32 {
				cw: 0,
				sw: 0,
				tag: 0,
				ipoff: 0,
				cssel: 0,
				dataoff: 0,
				datasel: 0,
				_st: [FpReg32 {
					significand: [0; 4],
					exponent: 0,
				}; 8],
				status: 0,
			},
			__ssp: [0; 4],
		}
	}

	/// Restores the context.
	pub fn restore_regs(&self, proc: &Process, frame: &mut IntFrame) {
		// Restore general registers
		frame.rax = self.uc_mcontext.gregs[GReg32::Eax as usize] as _;
		frame.rbx = self.uc_mcontext.gregs[GReg32::Ebx as usize] as _;
		frame.rcx = self.uc_mcontext.gregs[GReg32::Ecx as usize] as _;
		frame.rdx = self.uc_mcontext.gregs[GReg32::Edx as usize] as _;
		frame.rsi = self.uc_mcontext.gregs[GReg32::Esi as usize] as _;
		frame.rdi = self.uc_mcontext.gregs[GReg32::Edi as usize] as _;
		frame.rbp = self.uc_mcontext.gregs[GReg32::Ebp as usize] as _;
		// TODO restore fpstate
		proc.signal.lock().sigmask = self.uc_sigmask;
	}
}

/// 32-bit description of a signal stack.
#[repr(C)]
#[derive(Debug)]
pub struct Stack32 {
	pub ss_sp: u32, // 32 bit pointer
	pub ss_flags: i32,
	pub ss_size: u32,
}

/// 32-bit registers state.
#[repr(C)]
#[derive(Debug)]
pub struct MContext32 {
	pub gregs: [u32; 19],
	pub fpregs: u32, // 32 bit pointer
	pub oldmask: u32,
	pub cr2: u32,
}

/// 32-bit floating point registers state.
#[repr(C)]
#[derive(Debug)]
pub struct FpState32 {
	pub cw: u32,
	pub sw: u32,
	pub tag: u32,
	pub ipoff: u32,
	pub cssel: u32,
	pub dataoff: u32,
	pub datasel: u32,
	pub _st: [FpReg32; 8],
	pub status: u32,
}

/// TODO doc
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FpReg32 {
	pub significand: [u16; 4],
	pub exponent: u16,
}

// ------------------------------
//    64 bit structures

/// General purpose registers (64 bit).
#[cfg(target_arch = "x86_64")]
#[repr(usize)]
pub enum GReg64 {
	R8 = 0,
	R9,
	R10,
	R11,
	R12,
	R13,
	R14,
	R15,
	Rdi,
	Rsi,
	Rbp,
	Rbx,
	Rdx,
	Rax,
	Rcx,
	Rsp,
	Rip,
	Efl,
	Csgsfs,
	Err,
	Trapno,
	Oldmask,
	Cr2,
}

/// 64-bit userspace signal context.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug)]
pub struct UContext64 {
	pub uc_flags: u64,
	pub uc_link: u64, // 64 bit pointer
	pub uc_stack: Stack64,
	pub uc_mcontext: MContext64,
	pub uc_sigmask: SigSet,
	pub __fpregs_mem: FpState64,
	pub __ssp: [u64; 4],
}

#[cfg(target_arch = "x86_64")]
impl UContext64 {
	/// Creates a context structure from the current.
	pub fn new(process: &Process, frame: &IntFrame) -> Self {
		Self {
			uc_flags: 0, // TODO
			uc_link: 0,
			// TODO
			uc_stack: Stack64 {
				ss_sp: 0,
				ss_flags: 0,
				ss_size: 0,
			},
			uc_mcontext: MContext64 {
				gregs: [
					frame.r8,
					frame.r9,
					frame.r10,
					frame.r11,
					frame.r12,
					frame.r13,
					frame.r14,
					frame.r15,
					frame.rdi,
					frame.rsi,
					frame.rbp,
					frame.rbx,
					frame.rdx,
					frame.rax,
					frame.rcx,
					frame.rsp,
					frame.rip,
					frame.rflags,
					0, // TODO csgsfs
					0, // TODO err
					0, // TODO trapno
					0, // TODO oldmask
					0, // cr2
				],
				fpregs: 0, // TODO
				__reserved1: [0; 8],
			},
			uc_sigmask: process.signal.lock().sigmask,
			// TODO
			__fpregs_mem: FpState64 {
				cwd: 0,
				swd: 0,
				ftw: 0,
				fop: 0,
				rip: 0,
				rdp: 0,
				mxcsr: 0,
				mxcr_mask: 0,
				_st: [FpReg64 {
					significand: [0; 4],
					exponent: 0,
					__glibc_reserved1: [0; 3],
				}; 8],
				_xmm: [XmmReg64 {
					element: [0; 4],
				}; 16],
				__glibc_reserved1: [0; 24],
			},
			__ssp: [0; 4],
		}
	}

	/// Restores the context.
	pub fn restore_regs(&self, proc: &Process, frame: &mut IntFrame) {
		// Restore general registers
		frame.rax = self.uc_mcontext.gregs[GReg64::Rax as usize] as _;
		frame.rbx = self.uc_mcontext.gregs[GReg64::Rbx as usize] as _;
		frame.rcx = self.uc_mcontext.gregs[GReg64::Rcx as usize] as _;
		frame.rdx = self.uc_mcontext.gregs[GReg64::Rdx as usize] as _;
		frame.rsi = self.uc_mcontext.gregs[GReg64::Rsi as usize] as _;
		frame.rdi = self.uc_mcontext.gregs[GReg64::Rdi as usize] as _;
		frame.rbp = self.uc_mcontext.gregs[GReg64::Rbp as usize] as _;
		frame.r8 = self.uc_mcontext.gregs[GReg64::R8 as usize] as _;
		frame.r9 = self.uc_mcontext.gregs[GReg64::R9 as usize] as _;
		frame.r10 = self.uc_mcontext.gregs[GReg64::R10 as usize] as _;
		frame.r11 = self.uc_mcontext.gregs[GReg64::R11 as usize] as _;
		frame.r12 = self.uc_mcontext.gregs[GReg64::R12 as usize] as _;
		frame.r13 = self.uc_mcontext.gregs[GReg64::R13 as usize] as _;
		frame.r14 = self.uc_mcontext.gregs[GReg64::R14 as usize] as _;
		frame.r15 = self.uc_mcontext.gregs[GReg64::R15 as usize] as _;
		// TODO restore fpstate
		proc.signal.lock().sigmask = self.uc_sigmask;
	}
}

/// 64-bit description of a signal stack.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug)]
pub struct Stack64 {
	pub ss_sp: u64, // 64 bit pointer
	pub ss_flags: i32,
	pub ss_size: usize,
}

/// 64-bit registers state.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug)]
pub struct MContext64 {
	pub gregs: [u64; 23],
	pub fpregs: u64, // 64 bit pointer
	pub __reserved1: [u64; 8],
}

/// 64-bit floating point registers state.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug)]
pub struct FpState64 {
	pub cwd: u16,
	pub swd: u16,
	pub ftw: u16,
	pub fop: u16,
	pub rip: u64,
	pub rdp: u64,
	pub mxcsr: u32,
	pub mxcr_mask: u32,
	pub _st: [FpReg64; 8],
	pub _xmm: [XmmReg64; 16],
	pub __glibc_reserved1: [u32; 24],
}

/// TODO doc
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FpReg64 {
	pub significand: [u16; 4],
	pub exponent: u16,
	pub __glibc_reserved1: [u16; 3],
}

/// TODO doc
#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct XmmReg64 {
	pub element: [u32; 4],
}
