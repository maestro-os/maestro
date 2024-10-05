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
	memory::{vmem, PhysAddr, VirtAddr},
};
use core::arch::global_asm;

/// The physical address of the GDT.
const GDT_PHYS_ADDR: PhysAddr = PhysAddr(0x800);
/// The virtual address of the GDT.
const GDT_VIRT_ADDR: VirtAddr = VirtAddr(0xc0000800);

/// The initial Global Descriptor Table.
type InitGdt = [gdt::Entry; 9];

/// The initial Global Descriptor Table.
#[no_mangle]
#[link_section = ".boot.data"]
static INIT_GDT: InitGdt = [
	// First entry, empty
	gdt::Entry(0),
	// Kernel code segment
	gdt::Entry::new(0, !0, 0b10011010, 0b1100),
	// Kernel data segment
	gdt::Entry::new(0, !0, 0b10010010, 0b1100),
	// User code segment
	gdt::Entry::new(0, !0, 0b11111010, 0b1100),
	// User data segment
	gdt::Entry::new(0, !0, 0b11110010, 0b1100),
	// TSS
	gdt::Entry(0),
	// TLS entries
	gdt::Entry(0),
	gdt::Entry(0),
	gdt::Entry(0),
];

/// The page directory used to remap the kernel to higher memory.
///
/// The static is marked as **mutable** because the CPU will set the dirty flag.
#[no_mangle]
#[link_section = ".boot.data"]
static mut REMAP_DIR: vmem::x86::Table = const {
	use crate::vmem::x86::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE};
	use utils::limits::PAGE_SIZE;

	#[cfg(target_arch = "x86")]
	let mut dir = [0; 1024];
	#[cfg(target_arch = "x86_64")]
	let mut dir = [0; 512];
	// TODO use for loop when stabilized
	let mut i = 0;
	while i < 256 {
		#[cfg(target_arch = "x86")]
		let ent = {
			let addr = (i * PAGE_SIZE * 1024) as u32;
			addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT
		};
		#[cfg(target_arch = "x86_64")]
		let ent = {
			let addr = (i * PAGE_SIZE * 512 * 512) as u64;
			addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT
		};
		dir[i] = ent;
		dir[i + 256] = ent;
		i += 1;
	}
	vmem::x86::Table(dir)
};

extern "C" {
	/// The kernel's entry point.
	fn multiboot_entry();
}

// Common initialization code
global_asm!(
	r#"
.section .boot.text, "ax"

# Multiboot2 kernel header
.align 8
header:
	# Multiboot2 magic
	.long 0xe85250d6
	# Architecture (x86)
	.long 0
	# Header length
	.long (header_end - header)
	.long -(0xe85250d6 + (header_end - header))

# The entry tag, setting the entry point of the kernel.
.align 8
entry_address_tag:
	.short 3
	.short 0
	.long (entry_address_tag_end - entry_address_tag)
	.long multiboot_entry
entry_address_tag_end:

# End tag
.align 8
	.short 0
	.short 0
	.long 8
header_end:

.global multiboot_entry
.type multiboot_entry, @function

multiboot_entry:
	mov esp, offset boot_stack_begin
	xor ebp, ebp

	push ebx
	push eax
	call arch_setup
	call kernel_main
	# `kernel_main` cannot return
	ud2

.section .boot.stack, "aw"

.align 8

.set STACK_SIZE, 32768

boot_stack:
.size boot_stack, STACK_SIZE
.skip STACK_SIZE
boot_stack_begin:"#
);

// x86-specific initialization
#[cfg(target_arch = "x86")]
global_asm!(
	r#"
arch_setup:
	# Init flags
	push 0
	popfd

    # Copy GDT to its physical address
	mov esi, offset INIT_GDT
	mov edi, {GDT_PHYS_ADDR}
	mov ecx, {GDT_SIZE}
	rep movsb

	# Load GDT
	sub esp, 6
	mov word ptr [esp], ({GDT_SIZE} - 1)
	mov dword ptr [esp + 2], {GDT_PHYS_ADDR}
	lgdt [esp]
	add esp, 6
	mov eax, offset complete_flush
	push 8 # kernel code segment
	push eax
	retf
complete_flush:
	mov ax, 16 # kernel data segment
	mov ds, ax
	mov es, ax
	mov ss, ax

	mov ax, 0
	mov fs, ax
	mov gs, ax

    # Set page directory
    mov eax, offset {REMAP_DIR}
	mov cr3, eax

    # Enable PSE
	mov eax, cr4
	or eax, 0x00000010
	mov cr4, eax

    # Enable paging and write protect
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax

	# Update stack
    add esp, 0xc0000000

    # Update GDT
	sub esp, 6
	mov word ptr [esp], ({GDT_SIZE} - 1)
	mov dword ptr [esp + 2], {GDT_VIRT_ADDR}
	lgdt [esp]
	add esp, 6

	ret
"#,
	GDT_PHYS_ADDR = const(GDT_PHYS_ADDR.0),
	GDT_VIRT_ADDR = const(GDT_VIRT_ADDR.0),
	GDT_SIZE = const(size_of::<InitGdt>()),
	REMAP_DIR = sym REMAP_DIR
);

// x86_64-specific initialization
#[cfg(target_arch = "x86_64")]
global_asm!(
	r#"
arch_setup:
	# Init flags
	push 0
	popfq

    # Set page directory
    mov eax, offset {REMAP_DIR}
	mov cr3, eax

	# Enable PAE
	mov eax, cr4
	or eax, 0x20
	mov cr4, eax

	# Enable LME
	mov ecx, 0xc0000080 # EFER
	rdmsr
	or eax, 0x100
	wrmsr

    # Enable paging and write protect
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax
	
	# TODO init GDT

	ret
"#,
	REMAP_DIR = sym REMAP_DIR
);
