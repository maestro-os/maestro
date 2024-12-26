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

use crate::{
	elf,
	elf::parser::{SectionHeader, Sym},
	process::mem_space::bound_check,
};
use core::{intrinsics::unlikely, mem::size_of_val, ptr};

/// The name of the symbol pointing to the global offset table.
pub const GOT_SYM: &[u8] = b"_GLOBAL_OFFSET_TABLE_";

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
		self.get_info() >> 8
	}

	/// Returns the relocation type.
	fn get_type(&self) -> u8 {
		(self.get_info() & 0xff) as _
	}

	/// Returns the relocation's addend.
	fn get_addend(&self) -> isize {
		0
	}
}

/// Performs the relocation.
///
/// Arguments:
/// - `base_addr` is the base address at which the ELF is loaded.
/// - `rel_section` is the section containing the relocation.
/// - `get_sym` is a closure returning the a symbol. Arguments are:
///     - The index of the section containing the symbol.
///     - The index of the symbol in the section.
/// - `got` is the Global Offset Table's symbol (named after [`GOT_SYM`]).
///
/// If the relocation cannot be performed, the function returns an error.
///
/// # Safety
///
/// TODO
pub unsafe fn perform<R: Relocation, F>(
	relocation: &R,
	base_addr: *const u8,
	rel_section: &SectionHeader,
	get_sym: F,
	got: Option<&Sym>,
) -> Result<(), RelocationError>
where
	F: FnOnce(u32, usize) -> Option<usize>,
{
	let got_off = got.map(|sym| sym.st_value as usize).unwrap_or(0);
	// The address of the GOT
	let got_addr = (base_addr as usize).wrapping_add(got_off);
	// The offset in the GOT entry for the symbol
	let got_offset = 0usize; // TODO
						  // The offset in the PLT entry for the symbol
	let plt_offset = 0usize; // TODO
						  // The value of the symbol
	let sym_val = get_sym(rel_section.sh_link, relocation.get_sym());
	let value = match relocation.get_type() {
		elf::R_386_32 => sym_val
			.ok_or(RelocationError)?
			.wrapping_add_signed(relocation.get_addend()),
		elf::R_386_PC32 => sym_val
			.ok_or(RelocationError)?
			.wrapping_add_signed(relocation.get_addend())
			.wrapping_sub(relocation.get_offset()),
		elf::R_386_GOT32 => got_offset.wrapping_add_signed(relocation.get_addend()),
		elf::R_386_PLT32 => plt_offset
			.wrapping_add_signed(relocation.get_addend())
			.wrapping_sub(relocation.get_offset()),
		elf::R_386_COPY => return Ok(()),
		elf::R_386_GLOB_DAT | elf::R_386_JMP_SLOT => sym_val.unwrap_or(0),
		elf::R_386_RELATIVE => (base_addr as usize).wrapping_add_signed(relocation.get_addend()),
		elf::R_386_GOTOFF => sym_val
			.ok_or(RelocationError)?
			.wrapping_add_signed(relocation.get_addend())
			.wrapping_sub(got_addr),
		elf::R_386_GOTPC => got_addr
			.wrapping_add_signed(relocation.get_addend())
			.wrapping_sub(relocation.get_offset()),
		// Ignored
		elf::R_386_IRELATIVE => return Ok(()),
		_ => return Err(RelocationError),
	} as u32;
	let addr = base_addr.add(relocation.get_offset()) as *mut u32;
	// If the resulting address is in userspace, error
	if unlikely(bound_check(addr as _, size_of_val(&value))) {
		return Err(RelocationError);
	}
	let value = match relocation.get_type() {
		elf::R_386_RELATIVE => ptr::read_volatile(addr).wrapping_add(value),
		_ => value,
	};
	ptr::write_volatile(addr, value);
	Ok(())
}
