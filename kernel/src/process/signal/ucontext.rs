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

use crate::process::signal::SigSet;
use core::ffi::c_void;

// ------------------------------
//    32 bit structures

/// 32-bit userspace signal context.
#[repr(C)]
pub struct UContext32 {
	pub uc_flags: u32,
	pub uc_link: u32, // 32 bit pointer
	pub uc_stack: Stack32,
	pub uc_mcontext: MContext32,
	pub uc_sigmask: SigSet,
	pub __fpregs_mem: FpState32,
	pub __ssp: [u64; 4],
}

/// 32-bit description of a signal stack.
#[repr(C)]
pub struct Stack32 {
	pub ss_sp: u32, // 32 bit pointer
	pub ss_flags: i32,
	pub ss_size: u32,
}

/// 32-bit registers state.
#[repr(C)]
pub struct MContext32 {
	pub gregs: [u32; 19],
	pub fpregs: u32, // 32 bit pointer
	pub oldmask: u32,
	pub cr2: u32,
}

/// 32-bit floating point registers state.
#[repr(C)]
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
pub struct FpReg32 {
	pub significand: [u16; 4],
	pub exponent: u16,
}

// ------------------------------
//    64 bit structures

/// 64-bit userspace signal context.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct UContext64 {
	pub uc_flags: u64,
	pub uc_link: *mut Self,
	pub uc_stack: Stack64,
	pub uc_mcontext: MContext64,
	pub uc_sigmask: SigSet,
	pub __fpregs_mem: FpState64,
	pub __ssp: [u64; 4],
}

/// 64-bit description of a signal stack.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct Stack64 {
	pub ss_sp: *mut c_void,
	pub ss_flags: i32,
	pub ss_size: usize,
}

/// 64-bit registers state.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct MContext64 {
	pub gregs: [u64; 23],
	pub fpregs: *mut FpState64,
	pub __reserved1: [u64; 8],
}

/// 64-bit floating point registers state.
#[cfg(target_arch = "x86_64")]
#[repr(C)]
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
pub struct FpReg64 {
	pub significand: [u16; 4],
	pub exponent: u16,
	pub __glibc_reserved1: [u16; 3],
}

/// TODO doc
#[cfg(target_arch = "x86_64")]
#[repr(C)]
pub struct XmmReg64 {
	pub element: [u32; 4],
}
