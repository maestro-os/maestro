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

//! The `arch_prctl` system call sets architecture-specific thread state.

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::arch::x86;
use crate::{
	process::mem_space::copy::SyscallPtr,
	syscall::{Args, FromSyscallArg},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Set 64 bit base for the FS register.
const ARCH_SET_GS: c_int = 0x1001;
/// Set 64 bit base for the GS register.
const ARCH_SET_FS: c_int = 0x1002;
/// Get 64 bit base value for the FS register.
const ARCH_GET_FS: c_int = 0x1003;
/// Get 64 bit base value for the GS register.
const ARCH_GET_GS: c_int = 0x1004;
/// Tells whether the cpuid instruction is enabled.
const ARCH_GET_CPUID: c_int = 0x1011;
/// Enable or disable cpuid instruction.
const ARCH_SET_CPUID: c_int = 0x1012;

#[allow(unused_variables)]
pub fn arch_prctl(Args((code, addr)): Args<(c_int, usize)>) -> EResult<usize> {
	// For `gs`, use kernel base because it will get swapped when returning to userspace
	match code {
		#[cfg(target_arch = "x86_64")]
		ARCH_SET_GS => x86::wrmsr(x86::IA32_KERNEL_GS_BASE, addr as _),
		#[cfg(target_arch = "x86_64")]
		ARCH_SET_FS => x86::wrmsr(x86::IA32_FS_BASE, addr as _),
		#[cfg(target_arch = "x86_64")]
		ARCH_GET_FS => {
			let val = x86::rdmsr(x86::IA32_FS_BASE) as usize;
			let ptr = SyscallPtr::<usize>::from_ptr(addr);
			ptr.copy_to_user(&val)?;
		}
		#[cfg(target_arch = "x86_64")]
		ARCH_GET_GS => {
			let val = x86::rdmsr(x86::IA32_GS_BASE) as usize;
			let ptr = SyscallPtr::<usize>::from_ptr(addr);
			ptr.copy_to_user(&val)?;
		}
		// TODO ARCH_GET_CPUID
		// TODO ARCH_SET_CPUID
		_ => return Err(errno!(EINVAL)),
	}
	#[allow(unreachable_code)]
	Ok(0)
}
