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
	arch::{x86, x86::gdt},
	memory::VirtAddr,
};
use core::arch::global_asm;
use crate::arch::x86::paging::Table;

#[cfg(target_arch = "x86")]
pub const GDT_VIRT_ADDR: VirtAddr = VirtAddr(0xc0000800);
#[cfg(target_arch = "x86_64")]
pub const GDT_VIRT_ADDR: VirtAddr = VirtAddr(0xffff800000000800);

#[cfg(target_arch = "x86")]
pub type InitGdt = [gdt::Entry; 9];
#[cfg(target_arch = "x86_64")]
pub type InitGdt = [gdt::Entry; 11];

/// The initial Global Descriptor Table.
#[no_mangle]
#[link_section = ".boot.data"]
static INIT_GDT: InitGdt = [
	// First entry, empty
	gdt::Entry(0),
	// Kernel code segment
	#[cfg(target_arch = "x86")]
	gdt::Entry::new(0, !0, 0b10011010, 0b1100),
	#[cfg(target_arch = "x86_64")]
	gdt::Entry::new(0, !0, 0b10011010, 0b1110),
	// Kernel data segment
	#[cfg(target_arch = "x86")]
	gdt::Entry::new(0, !0, 0b10010010, 0b1100),
	#[cfg(target_arch = "x86_64")]
	gdt::Entry::new(0, !0, 0b10010010, 0b1110),
	// User code segment (32 bits)
	gdt::Entry::new(0, !0, 0b11111010, 0b1100),
	// User data segment (32 bits)
	gdt::Entry::new(0, !0, 0b11110010, 0b1100),
	// TSS
	gdt::Entry(0),
	// TLS entries
	gdt::Entry(0),
	gdt::Entry(0),
	gdt::Entry(0),
	// User code segment (64 bits)
	#[cfg(target_arch = "x86_64")]
	gdt::Entry::new(0, !0, 0b11111010, 0b1110),
	// User data segment (64 bits)
	#[cfg(target_arch = "x86_64")]
	gdt::Entry::new(0, !0, 0b11110010, 0b1110),
];

/// The paging object used to remap the kernel to higher memory.
///
/// The static is marked as **mutable** because the CPU will set the dirty flag.
#[no_mangle]
#[link_section = ".boot.data"]
static mut REMAP: Table = const {
	#[cfg(target_arch = "x86")]
	{
		use crate::arch::x86::paging::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE};
		use utils::limits::PAGE_SIZE;

		let mut dir = [0; 1024];
		// TODO use for loop when stabilized
		let mut i = 0;
		while i < 256 {
			let addr = (i * PAGE_SIZE * 1024) as u32;
			let ent = addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT;
			dir[i] = ent;
			dir[i + 768] = ent;
			i += 1;
		}
		Table(dir)
	}
	// This is initialized at runtime in assembly
	#[cfg(target_arch = "x86_64")]
	Table([0; 512])
};

/// Directory use for the stage 1 of kernel remapping to higher memory under `x86_64`.
///
/// This directory identity maps the first GiB of physical memory.
///
/// The static is marked as **mutable** because the CPU will set the dirty flag.
#[no_mangle]
#[link_section = ".boot.data"]
#[cfg(target_arch = "x86_64")]
static mut REMAP_DIR: Table = const {
	use crate::arch::x86::paging::{FLAG_PAGE_SIZE, FLAG_PRESENT, FLAG_WRITE};
	use utils::limits::PAGE_SIZE;

	let mut dir = [0; 512];
	// TODO use for loop when stabilized
	let mut i = 0;
	while i < dir.len() {
		let addr = (i * PAGE_SIZE * 512) as u64;
		let ent = addr | FLAG_PAGE_SIZE | FLAG_WRITE | FLAG_PRESENT;
		dir[i] = ent;
		i += 1;
	}
	Table(dir)
};

extern "C" {
	/// The kernel's entry point.
	fn multiboot_entry();
}

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

    # Copy GDT to its physical address
	mov esi, offset INIT_GDT
	mov edi, {GDT_VIRT_ADDR}
	mov ecx, {GDT_SIZE}
	rep movsb

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

gdt:
	.word {GDT_SIZE} - 1
	.long {GDT_VIRT_ADDR}
"#,
	GDT_VIRT_ADDR = const(GDT_VIRT_ADDR.0),
	GDT_SIZE = const(size_of::<InitGdt>()),
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
	or eax, 0x100
	wrmsr

    # Enable paging and write protect
	mov eax, cr0
	or eax, 0x80010000
	mov cr0, eax

    # Copy GDT to its address
	mov esi, offset INIT_GDT
	mov edi, {GDT_VIRT_ADDR}
	mov ecx, {GDT_SIZE}
	rep movsb

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

gdt:
	.word {GDT_SIZE} - 1
	.quad {GDT_VIRT_ADDR}
"#,
	GDT_VIRT_ADDR = const(GDT_VIRT_ADDR.0),
	GDT_SIZE = const(size_of::<InitGdt>()),
	REMAP = sym REMAP,
	REMAP_DIR = sym REMAP_DIR
);
