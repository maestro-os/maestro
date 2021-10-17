//! This module implements ELF relocations.

/// Trait implemented for relocation objects.
pub trait Relocation {
	/// Returns the `r_info` field of the relocation.
	fn get_info(&self) -> u32;

	/// Performs the relocation.
	unsafe fn perform();

	/// Returns the relocation's symbol.
	fn get_sym(&self) -> u32 {
		self.get_info() >> 8
	}

	/// Returns the relocation type.
	fn get_type(&self) -> u8 {
		(self.get_info() & 0xff) as _
	}
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
	fn get_info(&self) -> u32 {
		self.r_info
	}

	unsafe fn perform() {
		// TODO
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
	fn get_info(&self) -> u32 {
		self.r_info
	}

	unsafe fn perform() {
		// TODO
	}
}

/// TODO doc
fn perform_reloc(section: u32, offset: u32, sym: u32, type_: u8, addend: u32) {
	// The virtual address at which the image is located
	let base_addr = unsafe {
		mem.as_ptr() as u32
	};
	// The offset inside of the GOT
	let got_offset = 0; // TODO
	// The address of the GOT
	let got_addr = base_addr + match parser.get_symbol_by_name("_GLOBAL_OFFSET_TABLE_") {
		Some(sym) => sym.st_value,
		None => 0,
	};
	// The offset of the PLT entry for the symbol.
	let plt_offset = 0; // TODO

	// The value of the symbol
	// TODO Error on None?
	let sym_val = Self::get_symbol_value(&parser, base_addr as _, section, sym)
		.unwrap_or(0);

	let value = match type_ {
		elf::R_386_32 => sym_val + addend,
		elf::R_386_PC32 => sym_val + addend - offset,
		elf::R_386_GOT32 => got_offset + addend,
		elf::R_386_PLT32 => plt_offset + addend - offset,
		elf::R_386_GLOB_DAT | elf::R_386_JMP_SLOT => sym_val,
		elf::R_386_RELATIVE => base_addr + addend,
		elf::R_386_GOTOFF => sym_val + addend - got_addr,
		elf::R_386_GOTPC => got_addr + addend - offset,

		_ => {
			return;
		}
	};

	let addr = (base_addr + offset) as *mut u32;
	match type_ {
		elf::R_386_RELATIVE => unsafe {
			*addr += value;
		},

		_ => unsafe {
			*addr = value;
		},
	}
}
