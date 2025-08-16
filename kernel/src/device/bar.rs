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
use core::{mem::size_of, num::NonZeroUsize, ptr};

/// Enumeration of Memory Space BAR types.
#[derive(Clone, Debug)]
pub enum BARType {
	/// The base register is 32 bits wide.
	Size32,
	/// The base register is 64 bits wide.
	Size64,
}

/// Structure representing a Base Address Register.
#[derive(Clone, Debug)]
pub enum BAR {
	/// A memory mapped register.
	MemorySpace {
		/// The type of the BAR, specifying the size of the register.
		type_: BARType,
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

impl BAR {
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
	#[inline(always)]
	pub fn read<T>(&self, off: usize) -> u64 {
		match self {
			Self::MemorySpace {
				type_,
				address,
				..
			} => match type_ {
				BARType::Size32 => unsafe {
					let addr = address.add(off) as *const u32;
					ptr::read_volatile(addr).into()
				},
				BARType::Size64 => unsafe {
					let addr = address.add(off) as *const u64;
					ptr::read_volatile(addr)
				},
			},
			Self::IOSpace {
				address, ..
			} => {
				let off = address.wrapping_add(off as u32) as u16;
				unsafe {
					match size_of::<T>() {
						1 => inb(off).into(),
						2 => inw(off).into(),
						4 => inl(off).into(),
						_ => 0u32.into(),
					}
				}
			}
		}
	}

	/// Writes a value to the register at offset `off`.
	#[inline(always)]
	pub fn write<T>(&self, off: usize, val: u64) {
		match self {
			Self::MemorySpace {
				type_,
				address,
				..
			} => match type_ {
				BARType::Size32 => unsafe {
					let addr = address.add(off) as *mut u32;
					ptr::write_volatile(addr, val as _);
				},
				BARType::Size64 => unsafe {
					let addr = address.add(off) as *mut u64;
					ptr::write_volatile(addr, val);
				},
			},
			Self::IOSpace {
				address, ..
			} => {
				let off = address.wrapping_add(off as u32) as u16;
				unsafe {
					match size_of::<T>() {
						1 => outb(off, val as _),
						2 => outw(off, val as _),
						4 => outl(off, val as _),
						_ => {}
					}
				}
			}
		}
	}
}
