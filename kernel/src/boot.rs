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

use crate::{
	gdt,
	memory::{
		vmem::x86::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE},
		PhysAddr,
	},
};
use core::arch::global_asm;
use utils::limits::PAGE_SIZE;

/// The value of the Multiboot2 magic.
const MULTIBOOT_MAGIC: u32 = 0xe85250d6;

/// Multiboot header tag: End
const MULTIBOOT_HEADER_TAG_END: u16 = 0;
/// Multiboot header tag: The kernel's entry point address
const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS: u16 = 3;

/// The physical address of the GDT.
const GDT_PHYS_ADDR: PhysAddr = PhysAddr(0x800);

/// The header of a multiboot2 tag.
#[repr(C, align(8))]
struct MultibootTagHdr {
	/// The tag's type.
	r#type: u16,
	/// The tag's flags.
	///
	/// Currently, has only one flag:
	/// - `0`: if set, the tag may be considered as optional by the bootloader.
	flags: u16,
	/// The size of the tag in bytes.
	size: u32,
}

/// Layout of the multiboot2 tag indicating the entry point of the kernel.
#[repr(C, align(8))]
struct MultibootEntryAddrTag {
	/// The tag's header.
	hdr: MultibootTagHdr,
	/// The entry point's physical address.
	entry_addr: u32,
}

/// Layout of the multiboot2 header.
#[repr(C, align(8))]
struct MultibootHeader {
	// Mandatory fields
	/// Multiboot magic number.
	magic: u32,
	/// The CPU architecture to boot for.
	architecture: u32,
	/// The size of this header in bytes.
	header_length: u32,
	/// The checksum of the previous fields.
	checksum: u32,

	/// The entry point tag.
	entry_addr_tag: MultibootEntryAddrTag,
	/// The end tag.
	end_tag: MultibootTagHdr,
}

/// The header used to provide multiboot2 with the necessary information to boot the kernel.
#[no_mangle]
#[link_section = ".boot.rodata"]
pub static MULTIBOOT_HEADER: MultibootHeader = MultibootHeader {
	magic: MULTIBOOT_MAGIC,
	// x86
	architecture: 0,
	header_length: size_of::<MultibootHeader>() as _,
	// Compute checksum of the previous values
	checksum: 0.wrapping_sub(MULTIBOOT_MAGIC + 0 + size_of::<MultibootHeader>()),

	entry_addr_tag: MultibootEntryAddrTag {
		hdr: MultibootTagHdr {
			r#type: MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS,
			flags: 0,
			size: size_of::<MultibootEntryAddrTag>(),
		},
		entry_addr: multiboot_entry as _,
	},
	end_tag: MultibootTagHdr {
		r#type: MULTIBOOT_HEADER_TAG_END,
		flags: 0,
		size: size_of::<MultibootTagHdr>(),
	},
};

/// The initial Global Descriptor Table.
#[no_mangle]
#[link_section = ".boot.rodata"]
pub static INIT_GDT: [gdt::Entry; 9] = [
	// First entry, empty
	gdt::Entry::default(),
	// Kernel code segment
	gdt::Entry::new(0, !0, 0b10011010, 0b1100),
	// Kernel data segment
	gdt::Entry::new(0, !0, 0b10010010, 0b1100),
	// User code segment
	gdt::Entry::new(0, !0, 0b11111010, 0b1100),
	// User data segment
	gdt::Entry::new(0, !0, 0b11110010, 0b1100),
	// TSS
	gdt::Entry::default(),
	// TLS entries
	gdt::Entry::default(),
	gdt::Entry::default(),
	gdt::Entry::default(),
];

/// A page directory.
#[repr(C, align(8))]
struct PageDir([u32; 1024]);

impl PageDir {
	/// Initializes a page directory to remap the kernel to the higher half of the memory.
	pub const fn higher_half() -> Self {
		let mut dir = [0; 1024];
		for i in 0..256 {
			let addr = (i * PAGE_SIZE * 1024) as u32;
			let ent = addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT;
			dir[i] = ent;
			dir[i + 768] = ent;
		}
		Self(dir)
	}
}

/// The page directory used to remap the kernel to higher memory.
#[no_mangle]
#[link_section = ".boot.rodata"]
pub static REMAP_DIR: PageDir = PageDir::higher_half();

extern "C" {
	/// The kernel's entry point.
	fn multiboot_entry();
}

global_asm!(
	r"
.global multiboot_entry
.type multiboot_entry, @function

.section .boot.text

multiboot_entry:
	mov esp, boot_stack_begin
	xor ebp, ebp
	pushl 0
	popf

	push ebx
	push eax
	call setup_gdt
	call remap

	call kernel_main
	# `kernel_main` cannot return
	ud2

setup_gdt:
    # Copy GDT to its physical address
	mov esi, INIT_GDT
	mov edi, {GDT_PHYS_ADDR}
	mov ecx, {GDT_SIZE}
	rep movsb
	
	# Load GDT
	pushl {GDT_PHYS_ADDR}
	pushw ({GDT_SIZE} - 1)
	lgdt [esp]
	add esp, 6
	jmp 8, complete_flush
complete_flush:
	mov ax, GDT_KERNEL_DS
	mov ds, ax
	mov es, ax
	mov ss, ax

	mov ax, 0
	mov fs, ax
	mov gs, ax

	ret
	
/*
 * Remaps the first gigabyte of memory to the last one, enabling paging and PSE.
 */
remap:
    # Set page directory
	mov cr3, {REMAP_DIR_ADDR}
	
    # Enable PSE
	mov eax, cr4
	or eax, 0x00000010
	mov cr4, eax
	
    # Enable paging
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax
	
	# Update stack
    add esp, 0xc0000000
	
	ret

.section .boot.stack

.align 8

boot_stack:
.size boot_stack, STACK_SIZE
.skip STACK_SIZE
boot_stack_begin:
",
	GDT_SIZE = size_of_val(&INIT_GDT),
	REMAP_DIR_ADDR = &REMAP_DIR
);
