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

//! The Base Address Register (BAR) is a way to communicate with a device using
//! Direct Access Memory (DMA).

use crate::arch::x86::io::{inb, inl, inw, outb, outl, outw};
use core::{mem, mem::size_of, num::NonZeroUsize};

/// Position in physical memory where a BAR can be mapped
#[derive(Clone, Debug)]
pub enum BarType {
	/// Can be mapped in the 32 bit range
	Bit32,
	/// Can be mapped in the 64 bit range
	Bit64,
}

/// A Base Address Register
#[derive(Clone, Debug)]
pub enum Bar {
	/// A memory mapped register.
	MemorySpace {
		/// The type of the BAR, specifying the size of the register.
		type_: BarType,
		/// If `true`, read accesses do not have any side effects.
		prefetchable: bool,

		/// Pointer to the registers.
		address: *mut u8,
		/// The size of the address space in bytes.
		size: NonZeroUsize,
	},
	/// A IO port mapped register.
	IOSpace {
		/// Address to the registers in I/O space.
		address: u32,
		/// The size of the address space in bytes.
		size: usize,
	},
}

impl Bar {
	/// Returns the amount of memory.
	pub fn get_size(&self) -> usize {
		match self {
			Self::MemorySpace {
				size, ..
			} => size.get(),
			Self::IOSpace {
				size, ..
			} => *size,
		}
	}

	/// Tells whether the memory is prefetchable.
	pub fn is_prefetchable(&self) -> bool {
		match self {
			Self::MemorySpace {
				prefetchable, ..
			} => *prefetchable,
			Self::IOSpace {
				..
			} => false,
		}
	}

	/// Reads a value from the register at offset `off`.
	///
	/// # Safety
	///
	/// An invalid `off` results in an undefined behaviour.
	#[inline(always)]
	pub unsafe fn read<T>(&self, off: usize) -> T {
		match self {
			Self::MemorySpace {
				address, ..
			} => address.byte_add(off).cast::<T>().read_volatile(),
			Self::IOSpace {
				address, ..
			} => {
				let off = address.wrapping_add(off as u32) as u16;
				match size_of::<T>() {
					1 => mem::transmute_copy(&inb(off)),
					2 => mem::transmute_copy(&inw(off)),
					4 => mem::transmute_copy(&inl(off)),
					_ => mem::zeroed(),
				}
			}
		}
	}

	/// Writes a value to the register at offset `off`.
	///
	/// # Safety
	///
	/// An invalid `off` results in an undefined behaviour.
	#[inline(always)]
	pub unsafe fn write<T>(&self, off: usize, val: T) {
		match self {
			Self::MemorySpace {
				address, ..
			} => address.byte_add(off).cast::<T>().write_volatile(val),
			Self::IOSpace {
				address, ..
			} => {
				let off = address.wrapping_add(off as u32) as u16;
				match size_of::<T>() {
					1 => outb(off, mem::transmute_copy(&val)),
					2 => outw(off, mem::transmute_copy(&val)),
					4 => outl(off, mem::transmute_copy(&val)),
					_ => {}
				}
			}
		}
	}
}
