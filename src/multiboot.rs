/*
 * TODO doc
 */

use crate::memory::Void;
use crate::memory;

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

// TODO Check type
pub const MEMORY_AVAILABLE: u32 = 1;
pub const MEMORY_RESERVED: u32 = 2;
pub const MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MEMORY_NVS: u32 = 4;
pub const MEMORY_BADRAM: u32 = 5;

// TODO Check type
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
	addr: u64,
	len: u64,
	type_: u32,
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
	f0: TagFramebufferUnionF0,
	f1: TagFramebufferUnionF1,
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

/*
 * Structure representing the informations given to the kernel at boot time.
 */
pub struct BootInfo {
	/* TODO */
	pub cmdline: &'static str,
	/* TODO */
	pub loader_name: &'static str,

	/* TODO */
	pub mem_lower: u32,
	/* TODO */
	pub mem_upper: u32,
	/* TODO */
	pub memory_maps_size: usize,
	/* TODO */
	pub memory_maps_entry_size: usize,
	/* TODO */
	pub memory_maps: *const MmapEntry,

	/* TODO */
	pub elf_num: u32,
	/* TODO */
	pub elf_entsize: u32,
	/* TODO */
	pub elf_shndx: u32,
	/* TODO */
	pub phys_elf_sections: *const Void,
	/* TODO */
	pub elf_sections: *const Void,

	// TODO
}

/*
 * The field storing the informations given to the kernel at boot time.
 */
pub static mut BOOT_INFO: BootInfo = BootInfo {
	cmdline: "",
	loader_name: "",
	mem_lower: 0,
	mem_upper: 0,
	memory_maps_size: 0,
	memory_maps_entry_size: 0,
	memory_maps: 0 as *const _,
	elf_num: 0,
	elf_entsize: 0,
	elf_shndx: 0,
	phys_elf_sections: 0 as *const _,
	elf_sections: 0 as *const _,
};

/*
 * TODO
 */
fn tags_size(_ptr: *const Void) -> usize {
	// TODO
	0
}

/*
 * Reads the given `tag` and fills the boot informations structure accordingly.
 */
fn handle_tag(boot_info: &mut BootInfo, tag: *const Tag) {
	let type_ = unsafe { (*tag).type_ };
	match type_ {
		TAG_TYPE_CMDLINE => {
			/*let t = &tag as *const _ as *const TagString;
			let ptr = memory::kern_to_virt(&(*t).string as *const Void);
			boot_info.cmdline = &*(ptr as *const _ as *const [u8] as *const str);*/
			// TODO
		},

		TAG_TYPE_BOOT_LOADER_NAME => {
			// TODO
		},

		TAG_TYPE_MODULE => {
			// TODO
		},

		TAG_TYPE_BASIC_MEMINFO => {
			let t = &tag as *const _ as *const TagBasicMeminfo;
			unsafe {
				boot_info.mem_lower = (*t).mem_lower;
				boot_info.mem_upper = (*t).mem_upper;
			}
		},

		TAG_TYPE_BOOTDEV => {
			// TODO
		},

		TAG_TYPE_MMAP => {
			let t = &tag as *const _ as *const TagMmap;
			unsafe {
				boot_info.memory_maps_size = (*t).size as usize;
				boot_info.memory_maps_entry_size = (*t).entry_size as usize;
				let ptr = memory::kern_to_virt(&(*t).entries as *const _ as *const _);
				boot_info.memory_maps = ptr as *const _;
			}
		},

		TAG_TYPE_ELF_SECTIONS => {
			let t = &tag as *const _ as *const TagELFSections;
			unsafe {
				boot_info.elf_num = (*t).num;
				boot_info.elf_entsize = (*t).entsize;
				boot_info.elf_shndx = (*t).shndx;
				boot_info.phys_elf_sections = &(*t).sections as *const _;
				let ptr = memory::kern_to_virt(boot_info.phys_elf_sections as *const _);
				boot_info.elf_sections = ptr as *const _;
			}
		},

		// TODO

		_ => {}
	}
}

/*
 * Reads the multiboot tags from the given `ptr` and fills the boot informations structure.
 */
pub fn read_tags(ptr: *const Void) {
	unsafe {
		let mut tag = (ptr.offset(8)) as *const Tag;
		while (*tag).type_ != TAG_TYPE_END {
			handle_tag(&mut BOOT_INFO, tag);
			tag = (tag as *const u8).offset((((*tag).size + 7) & !7) as isize) as *const Tag;
		}
	}
}
