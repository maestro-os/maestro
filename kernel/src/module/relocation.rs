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

//! ELF kernel modules relocations implementation.

use crate::{elf::parser::SectionHeader, process::mem_space::bound_check};
use core::{hint::unlikely, ptr};

const R_386_NONE: u8 = 0;
const R_386_32: u8 = 1;
const R_386_PC32: u8 = 2;
const R_386_GOT32: u8 = 3;
const R_386_PLT32: u8 = 4;
const R_386_COPY: u8 = 5;
const R_386_GLOB_DAT: u8 = 6;
const R_386_JMP_SLOT: u8 = 7;
const R_386_RELATIVE: u8 = 8;
const R_386_GOTOFF: u8 = 9;
const R_386_GOTPC: u8 = 10;
const R_386_IRELATIVE: u8 = 42;

const R_X86_64_NONE: u8 = 0;
const R_X86_64_64: u8 = 1;
const R_X86_64_PC32: u8 = 2;
const R_X86_64_COPY: u8 = 5;
const R_X86_64_GLOB_DAT: u8 = 6;
const R_X86_64_JUMP_SLOT: u8 = 7;
const R_X86_64_RELATIVE: u8 = 8;

/// ELF relocation error.
pub struct RelocationError;

/// Trait implemented for relocation objects.
pub trait Relocation {
	/// The required section type for the relocation type.
	const REQUIRED_SECTION_TYPE: u32;

	/// Returns the `r_offset` field of the relocation.
	fn get_offset(&self) -> usize;

	/// Returns the `r_info` field of the relocation.
	fn get_info(&self) -> usize;

	/// Returns the relocation's symbol.
	fn get_sym(&self) -> usize {
		#[cfg(target_pointer_width = "32")]
		let shift = 8;
		#[cfg(target_pointer_width = "64")]
		let shift = 32;
		self.get_info() >> shift
	}

	/// Returns the relocation type.
	fn get_type(&self) -> u8 {
		#[cfg(target_pointer_width = "32")]
		let mask = 0xff;
		#[cfg(target_pointer_width = "64")]
		let mask = 0xffffffff;
		(self.get_info() & mask) as _
	}

	/// Returns the relocation's addend.
	///
	/// If an implicit addend is to be used, the function returns `None`.
	fn get_addend(&self) -> Option<isize> {
		None
	}
}

/// Performs the relocation for a kernel module.
///
/// Arguments:
/// - `rel` is the relocation.
/// - `base_addr` is the base address at which the ELF is loaded.
/// - `rel_section` is the section containing the relocation.
/// - `get_sym` is a closure returning the value of a symbol. Arguments are:
///     - The index of the section containing the symbol.
///     - The index of the symbol in the section.
///
/// If the relocation cannot be performed, the function returns an error.
///
/// # Safety
///
/// TODO
pub unsafe fn perform<R: Relocation, F>(
	rel: &R,
	base_addr: *mut u8,
	rel_section: &SectionHeader,
	get_sym: F,
) -> Result<(), RelocationError>
where
	F: FnOnce(u32, usize) -> Result<usize, RelocationError>,
{
	// Determine the size of the relocation
	#[cfg(target_pointer_width = "32")]
	let size = 4;
	#[cfg(target_pointer_width = "64")]
	let size = match rel.get_type() {
		R_X86_64_PC32 => 4,
		_ => 8,
	};
	// If the address is in userspace, error
	let addr = base_addr.add(rel.get_offset());
	if unlikely(bound_check(addr as _, size)) {
		return Err(RelocationError);
	}
	// Compute new value
	let get_sym = || get_sym(rel_section.sh_link, rel.get_sym());
	let get_addend = || {
		rel.get_addend().unwrap_or_else(|| {
			// Read implicit addend
			match size {
				4 => ptr::read_unaligned::<u32>(addr as _) as _,
				8 => ptr::read_unaligned::<u64>(addr as _) as _,
				_ => unreachable!(),
			}
		})
	};
	#[cfg(target_pointer_width = "32")]
	let value = match rel.get_type() {
		R_386_32 => get_sym()?.wrapping_add_signed(get_addend()),
		R_386_PC32 => get_sym()?
			.wrapping_add_signed(get_addend())
			.wrapping_sub(rel.get_offset()),
		R_386_GLOB_DAT | R_386_JMP_SLOT => get_sym()?,
		R_386_RELATIVE => (base_addr as usize).wrapping_add_signed(get_addend()),
		// Ignored
		R_386_NONE | R_386_COPY => return Ok(()),
		// Invalid or unsupported
		_ => return Err(RelocationError),
	};
	#[cfg(target_pointer_width = "64")]
	let value = match rel.get_type() {
		R_X86_64_64 => get_sym()?.wrapping_add_signed(get_addend()),
		R_X86_64_PC32 => get_sym()?
			.wrapping_add_signed(get_addend())
			.wrapping_sub(rel.get_offset()),
		R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => get_sym()?,
		R_X86_64_RELATIVE => (base_addr as usize).wrapping_add_signed(get_addend()),
		// Ignored
		R_X86_64_NONE | R_X86_64_COPY => return Ok(()),
		// Invalid or unsupported
		_ => return Err(RelocationError),
	};
	// Write value
	match size {
		4 => ptr::write_unaligned::<u32>(addr as _, value as _),
		8 => ptr::write_unaligned::<u64>(addr as _, value as _),
		_ => unreachable!(),
	}
	Ok(())
}
