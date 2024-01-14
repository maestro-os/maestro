/*
 * This file implements paging-related features.
 */

.section .text

.global paging_enable
.type paging_enable, @function

/*
 * (x86) Enables paging using the specified page directory.
 */
paging_enable:
	push %ebp
	mov %esp, %ebp
	push %eax

	mov 8(%ebp), %eax
	mov %eax, %cr3
	mov %cr0, %eax
	or $0x80010000, %eax
	mov %eax, %cr0

	pop %eax
	mov %ebp, %esp
	pop %ebp
	ret
