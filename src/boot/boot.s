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

/*
 * Constants used by Multiboot2 to detect the kernel.
 */
.set MULTIBOOT_MAGIC,			0xe85250d6
.set MULTIBOOT_ARCHITECTURE,	0
.set HEADER_LENGTH,				(header_end - header)
.set CHECKSUM,					-(MULTIBOOT_MAGIC + MULTIBOOT_ARCHITECTURE + HEADER_LENGTH)
/*
 * Multiboot header tags constants.
 */
.set MULTIBOOT_HEADER_TAG_END,					0
.set MULTIBOOT_HEADER_TAG_INFORMATION_REQUEST,	1
.set MULTIBOOT_HEADER_TAG_ADDRESS,				2
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS,		3
.set MULTIBOOT_HEADER_TAG_CONSOLE_FLAGS,		4
.set MULTIBOOT_HEADER_TAG_FRAMEBUFFER,			5
.set MULTIBOOT_HEADER_TAG_MODULE_ALIGN,			6
.set MULTIBOOT_HEADER_TAG_EFI_BS,				7
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI32,	8
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI64,	9
.set MULTIBOOT_HEADER_TAG_RELOCATABLE,			10

/*
 * The size of the kernel stack.
 */
.set STACK_SIZE,	32768

.global boot_stack
.global boot_stack_begin

.global multiboot_entry
.type multiboot_entry, @function

.extern setup_gdt
.extern _init
.extern _fini

.section .boot.text, "ax"

/*
 * The Multiboot2 kernel header.
 */
.align 8
header:
	.long MULTIBOOT_MAGIC
	.long MULTIBOOT_ARCHITECTURE
	.long HEADER_LENGTH
	.long CHECKSUM

/*
 * The entry tag, setting the entry point of the kernel.
 */
.align 8
entry_address_tag:
	.short MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS
	.short 1
	.long (entry_address_tag_end - entry_address_tag)
	.long multiboot_entry
entry_address_tag_end:

.align 8
	.short MULTIBOOT_HEADER_TAG_END
	.short 0
	.long 8
header_end:

/*
 * The entry point of the kernel.
 */
multiboot_entry:
	mov $boot_stack_begin, %esp
	xor %ebp, %ebp
	pushl $0
	popf
	cli

	push %eax
	push %ebx
	call a20_handle
	call setup_gdt
	call kernel_remap
	pop %ebx
	pop %eax

	mov $(0xc0000000 + boot_stack_begin), %esp
	push %ebx
	push %eax
	call kernel_main
	# `kernel_main` cannot return
	ud2



.section .boot.stack, "aw", @progbits

.align 8

/*
 * The kernel stack.
 */
boot_stack:
.size boot_stack, STACK_SIZE
.skip STACK_SIZE
boot_stack_begin:
