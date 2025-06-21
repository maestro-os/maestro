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

//! The Multiboot standard specifies an interface to load and boot the kernel
//! image. It provides essential information such as the memory mapping and the
//! ELF structure of the kernel.

use crate::{memory::PhysAddr, sync::once::OnceInit};
use core::{ffi::c_void, slice};

/// Multiboot2 magic number.
pub const BOOTLOADER_MAGIC: u32 = 0x36d76289;

/// Multiboot tag type: end of tags
pub const TAG_TYPE_END: u32 = 0;
/// Multiboot tag type: command line
pub const TAG_TYPE_CMDLINE: u32 = 1;
/// Multiboot tag type: bootloader name
pub const TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
/// Multiboot tag type: bootloader module
pub const TAG_TYPE_MODULE: u32 = 3;
/// Multiboot tag type: memory mapping information
pub const TAG_TYPE_BASIC_MEMINFO: u32 = 4;
/// Multiboot tag type: memory size
pub const TAG_TYPE_MMAP: u32 = 6;
/// Multiboot tag type: kernel's ELF sections
pub const TAG_TYPE_ELF_SECTIONS: u32 = 9;

/// Memory region: available
pub const MEMORY_AVAILABLE: u32 = 1;
/// Memory region: reserved
pub const MEMORY_RESERVED: u32 = 2;
/// Memory region: ACPI reclaimable
pub const MEMORY_ACPI_RECLAIMABLE: u32 = 3;
/// Memory region: ACPI NVS
pub const MEMORY_NVS: u32 = 4;
/// Memory region: bad memory
pub const MEMORY_BADRAM: u32 = 5;

/// A memory mapping entry.
#[repr(C)]
pub struct MmapEntry {
	/// The address to the beginning of the mapping.
	pub addr: u64,
	/// The length of the mapping.
	pub len: u64,
	/// Mapping type.
	pub type_: u32,
	zero: u32,
}

#[repr(C)]
struct Tag {
	type_: u32,
	size: u32,
}

#[repr(C)]
struct TagString {
	type_: u32,
	size: u32,
	string: [u8; 0],
}

#[repr(C)]
struct TagModule {
	type_: u32,
	size: u32,
	mod_start: u32,
	mod_end: u32,
	cmdline: [u8; 0],
}

#[repr(C)]
struct TagBasicMeminfo {
	type_: u32,
	size: u32,
	mem_lower: u32,
	mem_upper: u32,
}

#[repr(C)]
struct TagMmap {
	type_: u32,
	size: u32,
	entry_size: u32,
	entry_version: u32,
	entries: [MmapEntry; 0],
}

#[repr(C)]
struct TagELFSections {
	type_: u32,
	size: u32,
	num: u32,
	entsize: u32,
	shndx: u32,
	sections: [u8; 0],
}

impl MmapEntry {
	/// Tells if a Multiboot mmap entry is valid.
	pub fn is_valid(&self) -> bool {
		(self.addr + self.len) < (1_u64 << (4 * 8))
	}

	/// Returns the string describing the memory region according to its type.
	pub fn get_type_string(&self) -> &'static str {
		match self.type_ {
			MEMORY_AVAILABLE => "Available",
			MEMORY_ACPI_RECLAIMABLE => "ACPI",
			MEMORY_NVS => "Hibernate",
			MEMORY_BADRAM => "Bad RAM",
			_ => "Reserved",
		}
	}
}

/// Kernel boot information provided by Multiboot, structured and filtered.
#[derive(Default)]
pub struct BootInfo {
	/// The pointer to the end of the Multiboot2 tags.
	pub tags_end: PhysAddr,

	/// The command line used to boot the kernel.
	pub cmdline: Option<&'static [u8]>,
	/// The bootloader's name.
	pub loader_name: Option<&'static [u8]>,

	/// The upper memory size in kilobytes.
	pub mem_upper: u32,
	/// The size of physical memory mappings.
	pub memory_maps_size: usize,
	/// The size of a physical memory mapping entry.
	pub memory_maps_entry_size: usize,
	/// The list of physical memory mappings.
	pub memory_maps: *const MmapEntry,

	/// The number of ELF entries.
	pub elf_num: u32,
	/// The size of ELF entries.
	pub elf_entsize: u32,
	/// The index of the kernel's ELF section containing the kernel's symbols.
	pub elf_shndx: u32,
	/// The physical address of the kernel's ELF sections.
	pub elf_sections: PhysAddr,

	/// Slice of data representing an initramfs image.
	///
	/// If `None`, no initramfs is loaded.
	pub initramfs: Option<&'static [u8]>,
}

/// The field storing the information given to the kernel at boot time.
pub static BOOT_INFO: OnceInit<BootInfo> = unsafe { OnceInit::new() };

/// Reinterprets a tag with the given type.
unsafe fn reinterpret_tag<T>(tag: &Tag) -> &'static T {
	&*(tag as *const _ as *const T)
}

/// Reads the given `tag` and fills boot information structure accordingly.
fn handle_tag(boot_info: &mut BootInfo, tag: &Tag) {
	match tag.type_ {
		TAG_TYPE_CMDLINE => unsafe {
			let t: &TagString = reinterpret_tag(tag);
			let ptr = PhysAddr(t.string.as_ptr() as _)
				.kernel_to_virtual()
				.unwrap()
				.as_ptr();
			boot_info.cmdline = Some(utils::str_from_ptr(ptr));
		},
		TAG_TYPE_BOOT_LOADER_NAME => unsafe {
			let t: &TagString = reinterpret_tag(tag);
			let ptr = PhysAddr(t.string.as_ptr() as _)
				.kernel_to_virtual()
				.unwrap()
				.as_ptr();
			boot_info.loader_name = Some(utils::str_from_ptr(ptr));
		},
		TAG_TYPE_MODULE => {
			let data = unsafe {
				let t: &TagModule = reinterpret_tag(tag);
				let begin = PhysAddr(t.mod_start as _)
					.kernel_to_virtual()
					.unwrap()
					.as_ptr();
				let len = t.mod_end.saturating_sub(t.mod_start) as usize;
				slice::from_raw_parts::<u8>(begin, len)
			};
			boot_info.initramfs = (!data.is_empty()).then_some(data);
		}
		TAG_TYPE_BASIC_MEMINFO => {
			let t: &TagBasicMeminfo = unsafe { reinterpret_tag(tag) };
			boot_info.mem_upper = t.mem_upper;
		}
		TAG_TYPE_MMAP => {
			let t: &TagMmap = unsafe { reinterpret_tag(tag) };
			boot_info.memory_maps_size = t.size as usize;
			boot_info.memory_maps_entry_size = t.entry_size as usize;
			boot_info.memory_maps = t.entries.as_ptr();
		}
		TAG_TYPE_ELF_SECTIONS => {
			let t: &TagELFSections = unsafe { reinterpret_tag(tag) };
			boot_info.elf_num = t.num;
			boot_info.elf_entsize = t.entsize;
			boot_info.elf_shndx = t.shndx;
			boot_info.elf_sections = PhysAddr(t.sections.as_ptr() as usize);
		}
		_ => {}
	}
}

/// Returns the pointer to the next Multiboot tag after the current tag.
unsafe fn next(tag: *const Tag) -> *const Tag {
	let size = (*tag).size;
	tag.wrapping_byte_add(((size + 7) & !7) as usize)
}

/// Reads the multiboot tags from the given `ptr` and returns relevant information.
///
/// # Safety
///
/// The caller must ensure the given pointer is valid and points to Multiboot tags.
pub(crate) unsafe fn read(ptr: *const c_void) -> &'static BootInfo {
	let mut boot_info = BootInfo::default();
	let mut tag = ptr.offset(8) as *const Tag;
	while (*tag).type_ != TAG_TYPE_END {
		handle_tag(&mut boot_info, &*tag);
		tag = next(tag);
	}
	// Handle end tag
	tag = next(tag);
	boot_info.tags_end = PhysAddr(tag as _);
	// Write to static variable and return
	OnceInit::init(&BOOT_INFO, boot_info)
}
