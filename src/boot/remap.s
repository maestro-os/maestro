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
 * This file handles kernel remapping in order to place it in High Memory.
 * To do so, paging is enabled using a page directory that remaps the whole
 * kernel.
 *
 * The created page directory has to be replaced when kernel memory management
 * is ready.
 */

.section .boot.text, "ax"

.global kernel_remap

.type kernel_remap, @function
.type pse_enable, @function

.extern gdt_move

/*
 * Remaps the first gigabyte of memory to the last one.
 *
 * This function enables PSE.
 */
kernel_remap:
	push %ebx

	// Zero page directory
	xor %eax, %eax
	mov $remap_dir, %esi
L1:
	movl $0, (%esi)
	add $4, %esi
	add $1, %eax
	cmp $768, %eax
	jne L1

	// Fill entries
	xor %eax, %eax
	mov $remap_dir, %esi
L2:
	// (i * PAGE_SIZE * 1024)
	mov %eax, %ebx
	mov $22, %cl
	shl %cl, %ebx
	// PAGE_SIZE | WRITE | PRESENT
	or $(128 + 2 + 1), %ebx
	movl %ebx, (%esi)
	movl %ebx, (4 * 768)(%esi)
	add $4, %esi
	add $1, %eax
	cmp $256, %eax
	jne L2

	push $remap_dir
	call pse_enable
	add $4, %esp

	call gdt_move

	pop %ebx
	ret

/*
 * Enables Page Size Extension (PSE) and paging using the given page directory.
 */
pse_enable:
	push %ebp
	mov %esp, %ebp
	push %eax

	mov 8(%ebp), %eax
	mov %eax, %cr3

	mov %cr4, %eax
	or $0x00000010, %eax
	mov %eax, %cr4

	mov %cr0, %eax
	or $0x80010000, %eax
	mov %eax, %cr0

	pop %eax
	mov %ebp, %esp
	pop %ebp
	ret

.section .boot.data, "aw", @progbits

/*
 * The page directory used for kernel remapping.
 */
.align 4096
remap_dir:
.size remap_dir, 4096
.skip 4096
