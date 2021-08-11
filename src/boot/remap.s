.global pse_enable

.section .boot.text

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
