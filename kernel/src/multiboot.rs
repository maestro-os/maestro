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

use crate::memory;
use core::{ffi::c_void, mem::ManuallyDrop, ptr::null, slice};
use utils::lock::once::OnceInit;

pub const BOOTLOADER_MAGIC: u32 = 0x36d76289;
pub const TAG_ALIGN: usize = 8;

pub const TAG_TYPE_END: u32 = 0;
pub const TAG_TYPE_CMDLINE: u32 = 1;
pub const TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
pub const TAG_TYPE_MODULE: u32 = 3;
pub const TAG_TYPE_BASIC_MEMINFO: u32 = 4;
pub const TAG_TYPE_BOOTDEV: u32 = 5;
pub const TAG_TYPE_MMAP: u32 = 6;
pub const TAG_TYPE_VBE: u32 = 7;
pub const TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const TAG_TYPE_APM: u32 = 10;
pub const TAG_TYPE_EFI32: u32 = 11;
pub const TAG_TYPE_EFI64: u32 = 12;
pub const TAG_TYPE_SMBIOS: u32 = 13;
pub const TAG_TYPE_ACPI_OLD: u32 = 14;
pub const TAG_TYPE_ACPI_NEW: u32 = 15;
pub const TAG_TYPE_NETWORK: u32 = 16;
pub const TAG_TYPE_EFI_MMAP: u32 = 17;
pub const TAG_TYPE_EFI_BS: u32 = 18;
pub const TAG_TYPE_EFI32_IH: u32 = 19;
pub const TAG_TYPE_EFI64_IH: u32 = 20;
pub const TAG_TYPE_LOAD_BASE_ADDR: u32 = 21;

pub const MEMORY_AVAILABLE: u32 = 1;
pub const MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MEMORY_NVS: u32 = 4;
pub const MEMORY_BADRAM: u32 = 5;

pub const FRAMEBUFFER_TYPE_INDEXED: u32 = 0;
pub const FRAMEBUFFER_TYPE_RGB: u32 = 1;
pub const FRAMEBUFFER_TYPE_EGA_TEXT: u32 = 2;

#[repr(C)]
struct HeaderTag {
	type_: u16,
	flags: u16,
	size: u32,
}

#[repr(C)]
struct HeaderTagInformationRequest {
	type_: u16,
	flags: u16,
	size: u32,
	requests: [u32; 0],
}

#[repr(C)]
struct HeaderTagAddress {
	type_: u16,
	flags: u16,
	size: u32,
	header_addr: u32,
	load_addr: u32,
	load_end_addr: u32,
	bss_end_addr: u32,
}

#[repr(C)]
struct HeaderTagEntryAddress {
	type_: u16,
	flags: u16,
	size: u32,
	entry_addr: u32,
}

#[repr(C)]
struct HeaderTagConsoleFlags {
	type_: u16,
	flags: u16,
	size: u32,
	console_flags: u32,
}

#[repr(C)]
struct HeaderTagFramebuffer {
	type_: u16,
	flags: u16,
	size: u32,
	width: u32,
	height: u32,
	depth: u32,
}

#[repr(C)]
struct HeaderTagModuleAlign {
	type_: u16,
	flags: u16,
	size: u32,
}

#[repr(C)]
struct HeaderTagRelocatable {
	type_: u16,
	flags: u16,
	size: u32,
	min_addr: u32,
	max_addr: u32,
	align: u32,
	preference: u32,
}

#[repr(C)]
struct Color {
	red: u8,
	green: u8,
	blue: u8,
}

#[repr(C)]
pub struct MmapEntry {
	pub addr: u64,
	pub len: u64,
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
struct TagBootdev {
	type_: u32,
	size: u32,
	biosdev: u32,
	slice: u32,
	part: u32,
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
struct VBEInfoBlock {
	external_specification: [u8; 512],
}

#[repr(C)]
struct VBEModeInfoBlock {
	external_specification: [u8; 256],
}

#[repr(C)]
struct TagVBE {
	type_: u32,
	size: u32,

	vbe_mode: u16,
	vbe_interface_seg: u16,
	vbe_interface_off: u16,
	vbe_interface_len: u16,

	vbe_control_info: VBEInfoBlock,
	vbe_mode_info: VBEModeInfoBlock,
}

#[repr(C)]
struct TagFramebufferCommon {
	type_: u32,
	size: u32,

	framebuffer_addr: u64,
	framebuffer_pitch: u32,
	framebuffer_width: u32,
	framebuffer_height: u32,
	framebuffer_bpp: u8,
	framebuffer_type: u8,
	reserved: u16,
}

#[repr(C)]
struct TagFramebufferUnionF0 {
	framebuffer_palette_num_colors: u16,
	framebuffer_palette: [Color; 0],
}

#[repr(C)]
struct TagFramebufferUnionF1 {
	framebuffer_red_field_position: u8,
	framebuffer_red_mask_size: u8,
	framebuffer_green_field_position: u8,
	framebuffer_green_mask_size: u8,
	framebuffer_blue_field_position: u8,
	framebuffer_blue_mask_size: u8,
}

#[repr(C)]
union TagFramebufferUnion {
	f0: ManuallyDrop<TagFramebufferUnionF0>,
	f1: ManuallyDrop<TagFramebufferUnionF1>,
}

#[repr(C)]
struct TagFramebuffer {
	common: TagFramebufferCommon,
	u: TagFramebufferUnion,
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

#[repr(C)]
struct TagAPM {
	type_: u32,
	size: u32,
	version: u16,
	cseg: u16,
	offset: u32,
	cseg_16: u16,
	dseg: u16,
	flags: u16,
	cseg_len: u16,
	cseg_16_len: u16,
	dseg_len: u16,
}

#[repr(C)]
struct TagEFI32 {
	type_: u32,
	size: u32,
	pointer: u32,
}

#[repr(C)]
struct TagEFI64 {
	type_: u32,
	size: u32,
	pointer: u64,
}

#[repr(C)]
struct TagSMBIOS {
	type_: u32,
	size: u32,
	major: u8,
	minor: u8,
	reserved: [u8; 6],
	tables: [u8; 0],
}

#[repr(C)]
struct TagOldACPI {
	type_: u32,
	size: u32,
	rsdp: [u8; 0],
}

#[repr(C)]
struct TagNewACPI {
	type_: u32,
	size: u32,
	rsdp: [u8; 0],
}

#[repr(C)]
struct TagNetwork {
	type_: u32,
	size: u32,
	dhcpack: [u8; 0],
}

#[repr(C)]
struct TagEFIMmap {
	type_: u32,
	size: u32,
	descr_size: u32,
	descr_vers: u32,
	efi_mmap: [u8; 0],
}

#[repr(C)]
struct TagEFI32_IH {
	type_: u32,
	size: u32,
	pointer: u32,
}

#[repr(C)]
struct TagEFI64_IH {
	type_: u32,
	size: u32,
	pointer: u64,
}

#[repr(C)]
struct TagLoadBaseAddr {
	type_: u32,
	size: u32,
	load_base_addr: u32,
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

impl Tag {
	/// Returns the pointer to the next Multiboot tag after the current tag.
	pub fn next(&self) -> *const Self {
		((self as *const _ as usize) + (((self.size + 7) & !7) as usize)) as *const _
	}
}

/// Kernel boot information provided by Multiboot, structured and filtered.
pub struct BootInfo {
	/// The command line used to boot the kernel.
	pub cmdline: Option<&'static [u8]>,
	/// The bootloader's name.
	pub loader_name: Option<&'static [u8]>,

	/// The lower memory size.
	pub mem_lower: u32,
	/// The upper memory size.
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
	/// A pointer to the kernel's ELF sections.
	pub elf_sections: *const c_void,

	/// Slice of data representing an initramfs image.
	///
	/// If `None`, no initramfs is loaded.
	pub initramfs: Option<&'static [u8]>,
}

impl Default for BootInfo {
	fn default() -> Self {
		Self {
			cmdline: None,
			loader_name: None,
			mem_lower: 0,
			mem_upper: 0,
			memory_maps_size: 0,
			memory_maps_entry_size: 0,
			memory_maps: null(),
			elf_num: 0,
			elf_entsize: 0,
			elf_shndx: 0,
			elf_sections: null(),
			initramfs: None,
		}
	}
}

/// The field storing the information given to the kernel at boot time.
static BOOT_INFO: OnceInit<BootInfo> = unsafe { OnceInit::new() };

/// Returns boot information provided by Multiboot.
pub fn get_boot_info() -> &'static BootInfo {
	BOOT_INFO.get()
}

/// Reinterprets a tag with the given type.
unsafe fn reinterpret_tag<T>(tag: &Tag) -> &'static T {
	&*(tag as *const _ as *const T)
}

/// Reads the given `tag` and fills boot information structure accordingly.
fn handle_tag(boot_info: &mut BootInfo, tag: &Tag) {
	match tag.type_ {
		TAG_TYPE_CMDLINE => unsafe {
			let t: &TagString = reinterpret_tag(tag);
			let ptr = memory::kern_to_virt(t.string.as_ptr());
			boot_info.cmdline = Some(utils::str_from_ptr(ptr));
		},

		TAG_TYPE_BOOT_LOADER_NAME => unsafe {
			let t: &TagString = reinterpret_tag(tag);
			let ptr = memory::kern_to_virt(t.string.as_ptr());
			boot_info.loader_name = Some(utils::str_from_ptr(ptr));
		},

		TAG_TYPE_MODULE => {
			let data = unsafe {
				let t: &TagModule = reinterpret_tag(tag);
				let begin = memory::kern_to_virt(t.mod_start as *const u8);
				let end = memory::kern_to_virt(t.mod_end as *const u8);
				slice::from_raw_parts::<u8>(begin, end as usize - begin as usize)
			};
			boot_info.initramfs = (!data.is_empty()).then_some(data);
		}

		TAG_TYPE_BASIC_MEMINFO => {
			let t: &TagBasicMeminfo = unsafe { reinterpret_tag(tag) };
			boot_info.mem_lower = t.mem_lower;
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
			boot_info.elf_sections = t.sections.as_ptr() as _;
		}

		_ => {}
	}
}

/// Returns the size in bytes of Multiboot tags pointed to by `ptr`.
///
/// # Safety
///
/// The caller must ensure the given pointer is valid and points to Multiboot tags.
pub(crate) unsafe fn get_tags_size(ptr: *const c_void) -> usize {
	let mut tag = ptr.offset(8) as *const Tag;
	while (*tag).type_ != TAG_TYPE_END {
		tag = (*tag).next();
	}
	tag = (*tag).next();
	tag as usize - ptr as usize
}

/// Reads the multiboot tags from the given `ptr` and fills the boot
/// information structure.
///
/// # Safety
///
/// The caller must ensure the given pointer is valid and points to Multiboot tags.
pub(crate) unsafe fn read_tags(ptr: *const c_void) {
	let mut boot_info = BootInfo::default();
	let mut tag = ptr.offset(8) as *const Tag;
	while (*tag).type_ != TAG_TYPE_END {
		handle_tag(&mut boot_info, &*tag);
		tag = (*tag).next();
	}
	BOOT_INFO.init(boot_info);
}
