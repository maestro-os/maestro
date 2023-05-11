/*
 * This file handles kernel remapping in order to place it in High Memory.
 * To do so, paging is enabled using a page directory that remaps the whole
 * kernel.
 *
 * The created page directory has to be replaced when kernel memory management
 * is ready.
 */

.section .boot.text, "x"

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
	cmpl %eax, 768
	jne L1

	// Fill entries
	xor %eax, %eax
	mov $(remap_dir + (768 * 4)), %esi
L2:
	// (i * PAGE_SIZE * 1024)
	mov %eax, %ebx
	mov $22, %cl
	shl %cl, %ebx
	// PAGE_SIZE | WRITE | PRESENT
	or $(128 + 2 + 1), %ebx
	movl %ebx, (%esi)
	add $4, %esi
	add $1, %eax
	cmpl %eax, 256
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

.section .boot.data, "w", @progbits

/*
 * The page directory used for kernel remapping.
 */
.align 4096
remap_dir:
.size remap_dir, 4096
.skip 4096
