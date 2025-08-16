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

//! Functions to explore the kernel's ELF structures.

use super::SHT_SYMTAB;
use crate::{
	memory::{PhysAddr, VirtAddr},
	multiboot::BOOT_INFO,
	sync::once::OnceInit,
};
use core::{
	hint::unlikely,
	ptr,
	sync::atomic::{AtomicBool, Ordering::Relaxed},
};
use utils::{
	collections::hashmap::HashMap,
	errno::{AllocResult, CollectResult},
};

/// A kernel ELF section header.
#[cfg(target_arch = "x86")]
pub type KernSectionHeader = super::ELF32SectionHeader;
/// A kernel ELF section header.
#[cfg(target_arch = "x86_64")]
pub type KernSectionHeader = super::ELF64SectionHeader;

/// A kernel ELF symbol.
#[cfg(target_arch = "x86")]
pub type KernSym = super::ELF32Sym;
/// A kernel ELF symbol.
#[cfg(target_arch = "x86_64")]
pub type KernSym = super::ELF64Sym;

/// Tells whether global variables have been initialized.
pub(crate) static SYM_INIT: AtomicBool = AtomicBool::new(false);
/// A reference to the strtab.
static STRTAB: OnceInit<KernSectionHeader> = unsafe { OnceInit::new() };
/// Name-to-symbol map for the kernel.
static SYMBOLS: OnceInit<HashMap<&'static [u8], KernSym>> = unsafe { OnceInit::new() };

/// Returns an iterator over the kernel's ELF sections.
pub fn sections() -> impl Iterator<Item = KernSectionHeader> {
	(0..BOOT_INFO.elf_num).map(|i| get_section_by_offset(i).unwrap())
}

/// Returns a reference to the `n`th kernel section.
///
/// If the section does not exist, the function returns `None`.
pub fn get_section_by_offset(n: u32) -> Option<KernSectionHeader> {
	if n < BOOT_INFO.elf_num {
		let offset = n as usize * BOOT_INFO.elf_entsize as usize;
		let section = unsafe {
			let elf_sections = BOOT_INFO.elf_sections.kernel_to_virtual().unwrap();
			ptr::read_unaligned((elf_sections + offset).as_ptr())
		};
		Some(section)
	} else {
		None
	}
}

/// Returns the name of the given kernel ELF section.
///
/// If the name of the symbol could not be found, the function returns `None`.
pub fn get_section_name(section: &KernSectionHeader) -> Option<&'static [u8]> {
	// `unwrap` cannot fail because the ELF will always have this section
	let names_section = get_section_by_offset(BOOT_INFO.elf_shndx).unwrap();
	let ptr = PhysAddr(names_section.sh_addr as usize + section.sh_name as usize)
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// The string is in bound, otherwise the kernel's ELF is invalid
	Some(unsafe { utils::str_from_ptr(ptr) })
}

/// Returns a reference to the kernel section with name `name`.
///
/// `name` is the name of the required section.
///
/// If the section doesn't exist, the function returns `None`.
pub fn get_section_by_name(name: &[u8]) -> Option<KernSectionHeader> {
	sections().find(|s| get_section_name(s) == Some(name))
}

/// Returns an iterator over the kernel's ELF symbols.
pub fn symbols() -> impl Iterator<Item = KernSym> {
	let symtab = sections()
		.find(|section| section.sh_type == SHT_SYMTAB)
		.unwrap();
	let begin: *const u8 = PhysAddr(symtab.sh_addr as usize)
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	let symbols_count = (symtab.sh_size / symtab.sh_entsize) as usize;
	(0..symbols_count).map(move |i| {
		let off = i * symtab.sh_entsize as usize;
		unsafe { ptr::read_unaligned(begin.add(off) as *const KernSym) }
	})
}

/// Returns the name of the given kernel ELF symbol.
///
/// If the name of the symbol could not be found, the function returns `None`.
pub fn get_symbol_name(symbol: &KernSym) -> Option<&'static [u8]> {
	let ptr = PhysAddr(STRTAB.sh_addr as usize + symbol.st_name as usize)
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// The string is in bound, otherwise the kernel's ELF is invalid
	Some(unsafe { utils::str_from_ptr(ptr) })
}

/// Returns the name of the kernel function for the given instruction pointer.
///
/// `inst` is the pointer to the instruction on the virtual memory.
///
/// If the name cannot be retrieved, the function returns `None`.
pub fn get_function_name(inst: VirtAddr) -> Option<&'static [u8]> {
	// This function might be during a panic, which may happen before symbols are enumerated
	if unlikely(!SYM_INIT.load(Relaxed)) {
		return None;
	}
	symbols()
		.find(|sym| {
			let begin = VirtAddr(sym.st_value as usize);
			let end = begin + sym.st_size as usize;
			(begin..end).contains(&inst)
		})
		.as_ref()
		.and_then(get_symbol_name)
}

/// Returns the kernel symbol with the name `name`.
///
/// `name` is the name of the symbol to get.
///
/// If the symbol doesn't exist, the function returns `None`.
pub fn get_symbol_by_name(name: &[u8]) -> Option<&'static KernSym> {
	SYMBOLS.get(name)
}

/// Fills the kernel symbols map.
pub(crate) fn init() -> AllocResult<()> {
	// `.strtab` MUST be present
	// STRTAB must be initialized first because it is used to build the symbol map
	let strtab = get_section_by_name(b".strtab").unwrap();
	unsafe {
		OnceInit::init(&STRTAB, strtab);
	}
	// Build the symbol map
	let map = symbols()
		.map(|sym| {
			let name = get_symbol_name(&sym).unwrap_or(b"");
			(name, sym)
		})
		.collect::<CollectResult<_>>()
		.0?;
	unsafe {
		OnceInit::init(&SYMBOLS, map);
	}
	SYM_INIT.store(true, Relaxed);
	Ok(())
}
