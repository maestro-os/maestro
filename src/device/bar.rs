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

use crate::io;
use core::{mem::size_of, ptr};

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
	MemorySpace {
		/// The type of the BAR, specifying the size of the register.
		type_: BARType,
		/// If `true`, read accesses don't have any side effects.
		prefetchable: bool,

		/// Virtual address to the registers.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},

	IOSpace {
		/// Address to the registers in I/O space.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},
}

impl BAR {
	/// Returns the base address.
	pub fn get_address(&self) -> *mut () {
		match self {
			Self::MemorySpace {
				address, ..
			} => *address as _,
			Self::IOSpace {
				address, ..
			} => *address as _,
		}
	}

	/// Returns the amount of memory.
	pub fn get_size(&self) -> usize {
		match self {
			Self::MemorySpace {
				size, ..
			} => *size,
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
					let addr = (address + off as u64) as *const u32;
					ptr::read_volatile::<u32>(addr).into()
				},
				BARType::Size64 => unsafe {
					let addr = (address + off as u64) as *const u64;
					ptr::read_volatile::<u64>(addr)
				},
			},
			Self::IOSpace {
				address, ..
			} => {
				let off = (*address + off as u64) as u16;
				match size_of::<T>() {
					1 => unsafe { io::inb(off).into() },
					2 => unsafe { io::inw(off).into() },
					4 => unsafe { io::inl(off).into() },
					_ => 0u32.into(),
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
					let addr = (address + off as u64) as *mut u32;
					ptr::write_volatile::<u32>(addr, val as _);
				},
				BARType::Size64 => unsafe {
					let addr = (address + off as u64) as *mut u64;
					ptr::write_volatile::<u64>(addr, val);
				},
			},
			Self::IOSpace {
				address, ..
			} => {
				let off = (*address + off as u64) as u16;
				match size_of::<T>() {
					1 => unsafe { io::outb(off, val as _) },
					2 => unsafe { io::outw(off, val as _) },
					4 => unsafe { io::outl(off, val as _) },
					_ => {}
				}
			}
		}
	}
}
