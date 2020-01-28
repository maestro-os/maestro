.text

.global paging_enable
.global tlb_reload
.global cr2_get
.global cr3_get
.global paging_disable

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
 * (x86) Reloads the Translate Lookaside Buffer.
 */
tlb_reload:
	push %eax
	movl %cr3, %eax
	movl %eax, %cr3
	pop %eax
	ret

/*
 * (x86) Returns the value inside of the %cr2 register. When a Page Fault
 * occurs, this register is set with the linear address that was accessed.
 */
cr2_get:
	mov %cr2, %eax
	ret

/*
 * (x86) Returns the value inside of the %cr3 register. This register contains
 * the pointer to the current page directory.
 */
cr3_get:
	mov %cr3, %eax
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
