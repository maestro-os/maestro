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

//! The memory is one of the main component of the system.
//!
//! This module handles almost every memory-related features, including physical
//! memory map retrieving, memory allocation, virtual memory management, ...
//!
//! The system's memory is divided in two chunks:
//! - Userspace: Virtual memory below `PROCESS_END`, used by the currently running process
//! - Kernelspace: Virtual memory above `KERNEL_BEGIN`, used by the kernel itself and shared across
//!   processes

use crate::syscall::FromSyscallArg;
use core::{
	fmt,
	mem::size_of,
	ops::{Add, Deref, DerefMut, Sub},
	ptr,
	ptr::NonNull,
};

pub mod alloc;
pub mod buddy;
pub mod malloc;
pub mod memmap;
pub mod mmio;
pub mod stats;
#[cfg(feature = "memtrace")]
mod trace;
pub mod vmem;

/// Address of the beginning of the allocatable region in the virtual memory.
pub const ALLOC_BEGIN: VirtAddr = VirtAddr(0x40000000);
/// Address of the end of the virtual memory reserved to the process.
#[cfg(target_arch = "x86")]
pub const PROCESS_END: VirtAddr = VirtAddr(0xc0000000);
/// Address of the end of the virtual memory reserved to the process.
#[cfg(target_arch = "x86_64")]
pub const PROCESS_END: VirtAddr = VirtAddr(0x800000000000);

/// Address of the beginning of the kernelspace.
#[cfg(not(target_arch = "x86_64"))]
pub const KERNEL_BEGIN: VirtAddr = PROCESS_END;
/// Address of the beginning of the kernelspace.
#[cfg(target_arch = "x86_64")]
pub const KERNEL_BEGIN: VirtAddr = VirtAddr(0xffff800000000000);

/// The size of the kernelspace virtual memory in bytes.
pub const KERNELSPACE_SIZE: usize = usize::MAX - KERNEL_BEGIN.0 + 1;

/// An address on physical memory.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
	/// Converts the kernel physical address to a virtual address.
	///
	/// If the address is outside the kernelspace, the function returns `None`.
	pub fn kernel_to_virtual(self) -> Option<VirtAddr> {
		self.0.checked_add(KERNEL_BEGIN.0).map(VirtAddr)
	}
}

/// An address on virtual memory.
///
/// This would usually be represented by a pointer. However, in some cases we need to be able to
/// represent virtual addresses without having to dereference them.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VirtAddr(pub usize);

impl<T> From<*const T> for VirtAddr {
	fn from(ptr: *const T) -> Self {
		Self(ptr as usize)
	}
}

impl<T> From<*mut T> for VirtAddr {
	fn from(ptr: *mut T) -> Self {
		Self(ptr as usize)
	}
}

impl<T> From<NonNull<T>> for VirtAddr {
	fn from(ptr: NonNull<T>) -> Self {
		Self(ptr.as_ptr() as usize)
	}
}

impl FromSyscallArg for VirtAddr {
	fn from_syscall_arg(ptr: usize, _compat: bool) -> Self {
		Self(ptr)
	}
}

impl VirtAddr {
	/// Converts the kernel virtual address to a physical address.
	///
	/// If the address is outside the kernelspace, the function returns `None`.
	pub fn kernel_to_physical(self) -> Option<PhysAddr> {
		self.0.checked_sub(KERNEL_BEGIN.0).map(PhysAddr)
	}

	/// Returns a mutable pointer to the virtual address.
	///
	/// Underneath, this function uses [`ptr::with_exposed_provenance_mut`].
	pub fn as_ptr<T>(self) -> *mut T {
		ptr::with_exposed_provenance_mut(self.0)
	}
}

macro_rules! addr_impl {
	($name:ident) => {
		impl $name {
			/// Tells whether the pointer is null.
			pub fn is_null(self) -> bool {
				self.0 == 0
			}

			/// Tells whether the pointer is aligned to `align`.
			pub fn is_aligned_to(self, align: usize) -> bool {
				self.0 % align == 0
			}

			/// Computes and returns the next address to be aligned to `align`.
			///
			/// If `self` is already aligned, the function returns `self`.
			pub fn align_to(self, align: usize) -> Self {
				Self(self.0.next_multiple_of(align))
			}

			/// Computes and returns the previous address to be aligned to `align`.
			///
			/// If `self` is already aligned, the function returns `self`.
			pub fn down_align_to(self, align: usize) -> Self {
				Self(self.0 & !(align - 1))
			}
		}

		impl Deref for $name {
			type Target = usize;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl DerefMut for $name {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.0
			}
		}

		impl Add<usize> for $name {
			type Output = Self;

			/// Adds the given offset in bytes, wrapping on overflow.
			fn add(self, off: usize) -> Self::Output {
				Self(self.0.wrapping_add(off))
			}
		}

		impl Sub<usize> for $name {
			type Output = Self;

			/// Subtracts the given offset in bytes, wrapping on overflow.
			fn sub(self, off: usize) -> Self::Output {
				Self(self.0.wrapping_sub(off))
			}
		}

		impl fmt::Debug for $name {
			fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
				const LEN: usize = size_of::<usize>() * 2;
				write!(fmt, "{:0LEN$x}", self.0)
			}
		}
	};
}

addr_impl!(PhysAddr);
addr_impl!(VirtAddr);
