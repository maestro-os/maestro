.text

.global cr0_get
.global cr0_set
.global cr0_clear
.global cr2_get
.global cr3_get

.global paging_enable
.global paging_disable
.global tlb_reload

/*
 * (x86) Returns the value inside of the %cr0 register. This register contains
 * some flags for the processor.
 */
cr0_get:
	mov %cr0, %eax
	ret

/*
 * (x86) Sets the given flags in the %cr0 register.
 */
cr0_set:
	push %eax
	mov %cr0, %eax
	or 8(%esp), %eax
	mov %eax, %cr0
	pop %eax
	ret

/*
 * (x86) Clears the given flags in the %cr0 register.
 */
cr0_clear:
	push %eax
	push %ebx
	mov %cr0, %eax
	mov 12(%esp), %ebx
	not %ebx
	and %ebx, %eax
	mov %eax, %cr0
	pop %ebx
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
 * (x86) Reloads the Translate Lookaside Buffer.
 */
tlb_reload:
	push %eax
	movl %cr3, %eax
	movl %eax, %cr3
	pop %eax
	ret
