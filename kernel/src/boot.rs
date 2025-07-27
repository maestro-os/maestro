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

use crate::arch::x86::paging::Table;
use core::{arch::global_asm, sync::atomic::AtomicUsize};

/// Boot stack size
#[cfg(debug_assertions)]
pub const BOOT_STACK_SIZE: usize = 262144; // rustc in debug mode is greedy
/// Boot stack size
#[cfg(not(debug_assertions))]
pub const BOOT_STACK_SIZE: usize = 32768;

/// The paging object used to remap the kernel to higher memory.
///
/// The static is marked as **mutable** because the CPU will set the dirty flag.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".boot.data")]
static mut REMAP: Table = const {
	#[cfg(target_arch = "x86")]
	{
		use crate::arch::x86::paging::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE};
		use utils::limits::PAGE_SIZE;

		let mut dir = Table::new();
		// TODO use for loop when stabilized
		let mut i = 0;
		while i < 256 {
			let addr = i * PAGE_SIZE * 1024; // 4 MB entry
			let ent = addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT;
			dir.0[i] = AtomicUsize::new(ent);
			dir.0[i + 768] = AtomicUsize::new(ent);
			i += 1;
		}
		dir
	}
	// This is initialized at runtime in assembly
	#[cfg(target_arch = "x86_64")]
	Table::new()
};

/// Directory use for the stage 1 of kernel remapping to higher memory under `x86_64`.
///
/// This directory identity maps the first 512 GiB of physical memory.
///
/// The static is marked as **mutable** because the CPU will set the dirty flag.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".boot.data")]
#[cfg(target_arch = "x86_64")]
static mut REMAP_DIR: Table = const {
	use crate::arch::x86::paging::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE};
	use utils::limits::PAGE_SIZE;

	let mut dir = Table::new();
	// TODO use for loop when stabilized
	let mut i = 0;
	while i < dir.0.len() {
		let addr = i * PAGE_SIZE * 512 * 512; // 1 GB entry
		dir.0[i] = AtomicUsize::new(addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT);
		i += 1;
	}
	dir
};

// Common initialization code
global_asm!(
	r#"
.code32
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

.section .boot.stack, "aw"

.align 8

boot_stack:
.size boot_stack, {BOOT_STACK_SIZE}
.skip {BOOT_STACK_SIZE}
boot_stack_begin:
"#,
	BOOT_STACK_SIZE = const(BOOT_STACK_SIZE)
);

// x86-specific initialization
#[cfg(target_arch = "x86")]
global_asm!(
	r#"
.section .boot.text

.global multiboot_entry
.hidden complete_flush
.type multiboot_entry, @function

multiboot_entry:
	mov esp, offset boot_stack_begin
	xor ebp, ebp
	push 0
	popfd

	# Stash multiboot info
	push ebx
	push eax

    # Set page directory
    mov eax, offset {REMAP}
	mov cr3, eax

    # Enable PSE
	mov eax, cr4
	or eax, 0x10
	mov cr4, eax

    # Enable paging and write protect
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax

	# Load GDT
	lgdt [gdt]
	push 8 # kernel code segment
	mov eax, offset complete_flush
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

	# Update stack
    add esp, 0xc0000000

	call kernel_main
	# cannot return
	ud2

.section .boot.data

.align 8
gdt_entries:
	.long 0, 0
	.long 0x0000ffff, 0x00cf9a00 # code
	.long 0x0000ffff, 0x00cf9200 # data
gdt:
	.word gdt - gdt_entries - 1
	.long 0xc0000000 + gdt_entries
"#,
	REMAP = sym REMAP
);

// x86_64-specific initialization
#[cfg(target_arch = "x86_64")]
global_asm!(
	r#"
.code32
.section .boot.text

.global multiboot_entry
.hidden complete_flush
.type multiboot_entry, @function

multiboot_entry:
	mov esp, offset boot_stack_begin
	xor ebp, ebp
	push 0
	popfd

	# Stash multiboot info
	push ebx
	push eax

	# Init PDPT (offset 0 and 256)
	mov eax, offset {REMAP_DIR}
	or eax, 0b11 # address | WRITE | PRESENT
	mov {REMAP}, eax
	mov dword ptr [offset {REMAP} + 256 * 8], eax

    # Set PDPT
    mov eax, offset {REMAP}
	mov cr3, eax

	# Enable PSE and PAE
	mov eax, cr4
	or eax, 0x30
	mov cr4, eax

	# Enable LME
	mov ecx, 0xc0000080 # EFER
	rdmsr
	or eax, 0x901
	wrmsr

    # Enable paging and write protect
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax

	# Load GDT
	lgdt [gdt]
	push 8 # kernel code segment
	mov eax, offset complete_flush
	push eax
	retf
complete_flush:
.code64
	mov ax, 16 # kernel data segment
	mov ds, ax
	mov es, ax
	mov ss, ax

	mov ax, 0
	mov fs, ax
	mov gs, ax

	# Update stack and GDT
	mov rax, 0xffff800000000000
    add rsp, rax
    lgdt [gdt]

	# Call kernel_main
	xor rdi, rdi
	mov edi, dword ptr [rsp]
	xor rsi, rsi
	mov esi, dword ptr [rsp + 4]
	add rsp, 8
	movabs rax, offset kernel_main
	call rax
	# cannot return
	ud2

.section .boot.data

.align 8
gdt_entries:
	.long 0, 0
	.long 0x0000ffff, 0x00af9a00 # code
	.long 0x0000ffff, 0x008f9200 # data
gdt:
	.word gdt - gdt_entries - 1
	.quad 0xffff800000000000 + gdt_entries
"#,
	REMAP = sym REMAP,
	REMAP_DIR = sym REMAP_DIR
);
