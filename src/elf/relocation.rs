//! This module implements ELF relocations.

use crate::elf;
use crate::elf::ELF32SectionHeader;
use crate::elf::ELF32Sym;
use core::ffi::c_void;
use core::ptr;

/// The name of the symbol pointing to the global offset table.
const GOT_SYM: &str = "_GLOBAL_OFFSET_TABLE_";

/// Trait implemented for relocation objects.
pub trait Relocation {
	/// Returns the `r_offset` field of the relocation.
	fn get_offset(&self) -> u32;

	/// Returns the `r_info` field of the relocation.
	fn get_info(&self) -> u32;

	/// Performs the relocation.
	///
	/// Arguments:
	/// - `base_addr` is the base address at which the ELF is loaded.
	/// - `rel_section` is the section containing the relocation.
	/// - `get_sym` is a closure returning a symbol from its name.
	/// - `get_sym_val` is a closure returning the value of a symbol. Arguments are:
	///     - The index of the section containing the symbol.
	///     - The index of the symbol in the section.
	///
	/// If the relocation cannot be performed, the function returns an error.
	unsafe fn perform<'a, F0, F1>(
		&self,
		base_addr: *const c_void,
		rel_section: &ELF32SectionHeader,
		get_sym: F0,
		get_sym_val: F1,
	) -> Result<(), ()>
	where
		F0: FnOnce(&str) -> Option<&'a ELF32Sym>,
		F1: FnOnce(u32, u32) -> Option<u32>,
	{
		let got_off = get_sym(GOT_SYM).map(|sym| sym.st_value).unwrap_or(0);
		// The address of the GOT
		let got_addr = (base_addr as u32).wrapping_add(got_off);

		// The offset of the GOT entry for the symbol
		let got_offset = 0u32; // TODO
					   // The offset of the PLT entry for the symbol
		let plt_offset = 0u32; // TODO

		// The value of the symbol
		let sym_val = get_sym_val(rel_section.sh_link, self.get_sym());

		let value = match self.get_type() {
			elf::R_386_32 => sym_val.ok_or(())?.wrapping_add(self.get_addend()),
			elf::R_386_PC32 => sym_val
				.ok_or(())?
				.wrapping_add(self.get_addend())
				.wrapping_sub(self.get_offset()),
			elf::R_386_GOT32 => got_offset.wrapping_add(self.get_addend()),
			elf::R_386_PLT32 => plt_offset
				.wrapping_add(self.get_addend())
				.wrapping_sub(self.get_offset()),
			elf::R_386_COPY => return Ok(()),
			elf::R_386_GLOB_DAT | elf::R_386_JMP_SLOT => sym_val.unwrap_or(0),
			elf::R_386_RELATIVE => (base_addr as u32).wrapping_add(self.get_addend()),
			elf::R_386_GOTOFF => sym_val
				.ok_or(())?
				.wrapping_add(self.get_addend())
				.wrapping_sub(got_addr),
			elf::R_386_GOTPC => got_addr
				.wrapping_add(self.get_addend())
				.wrapping_sub(self.get_offset()),

			// Ignored relocations
			elf::R_386_IRELATIVE => return Ok(()),

			_ => return Err(()),
		};

		let addr = (base_addr as u32).wrapping_add(self.get_offset()) as *mut u32;
		// TODO Check the address is accessible

		let value = match self.get_type() {
			elf::R_386_RELATIVE => ptr::read_volatile(addr).wrapping_add(value),
			_ => value,
		};
		ptr::write_volatile(addr, value);

		Ok(())
	}

	/// Returns the relocation's symbol.
	fn get_sym(&self) -> u32 {
		self.get_info() >> 8
	}

	/// Returns the relocation type.
	fn get_type(&self) -> u8 {
		(self.get_info() & 0xff) as _
	}

	/// Returns the relocation's addend.
	fn get_addend(&self) -> u32;
}

/// Structure representing an ELF relocation.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF32Rel {
	/// The location of the relocation action.
	pub r_offset: u32,
	/// The relocation type and symbol index.
	pub r_info: u32,
}

impl Relocation for ELF32Rel {
	fn get_offset(&self) -> u32 {
		self.r_offset
	}

	fn get_info(&self) -> u32 {
		self.r_info
	}

	fn get_addend(&self) -> u32 {
		0
	}
}

/// Structure representing an ELF relocation with an addend.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF32Rela {
	/// The location of the relocation action.
	pub r_offset: u32,
	/// The relocation type and symbol index.
	pub r_info: u32,
	/// A constant value used to compute the relocation.
	pub r_addend: u32,
}

impl Relocation for ELF32Rela {
	fn get_offset(&self) -> u32 {
		self.r_offset
	}

	fn get_info(&self) -> u32 {
		self.r_info
	}

	fn get_addend(&self) -> u32 {
		self.r_addend
	}
}
