/*
 * This file implements paging-related features.
 */

.section .text

.global paging_enable
.global paging_disable
.global invlpg
.global tlb_reload

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

/*
 * (x86) Disables paging.
 */
paging_disable:
	push %eax
	mov %cr0, %eax
	and $(~0x80000000), %eax
	mov %eax, %cr0
	pop %eax
	ret

/*
 * (x86) Executes the invlpg for the given page address.
 */
invlpg:
	push %eax

	mov 4(%esp), %eax
	invlpg (%eax)

	pop %eax
	ret

/*
 * (x86) Reloads the Translate Lookaside Buffer.
 */
tlb_reload:
	push %eax

	movl %cr3, %eax
	movl %eax, %cr3

	pop %eax
	ret
