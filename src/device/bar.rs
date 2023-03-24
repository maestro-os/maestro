//! The Base Address Register (BAR) is a way to communicate with a device using
//! Direct Access Memory (DMA).

use core::mem::size_of;
use core::ptr;
use crate::io;

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

		/// Physical address to the register.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},

	IOSpace {
		/// Address to the register in I/O space.
		address: u64,

		/// The size of the address space in bytes.
		size: usize,
	},
}

impl BAR {
	/// Returns the base address.
	pub fn get_physical_address(&self) -> Option<*mut ()> {
		let (addr, size) = match self {
			Self::MemorySpace {
				address,
				size,
				..
			} => (*address, *size),
			Self::IOSpace {
				address,
				size,
				..
			} => (*address, *size),
		};

		if (addr + size as u64) > usize::MAX as u64 {
			Some(addr as _)
		} else {
			None
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

	// TODO Use virtual addresses instead
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

	// TODO Use virtual addresses instead
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
