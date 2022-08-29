/*
 * This file implements access to x86 control registers.
 */

.global cr0_get
.global cr0_set
.global cr0_clear
.global cr2_get
.global cr3_get
.global cr4_get
.global cr4_set

/*
 * (x86) Returns the value of the %cr0 register.
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
 * (x86) Returns the value of the %cr2 register. When a Page Fault occurs, this register is set
 * with the linear address that caused it.
 */
cr2_get:
	mov %cr2, %eax
	ret

/*
 * (x86) Returns the value of the %cr3 register. This register contains the pointer to the current
 * page directory.
 */
cr3_get:
	mov %cr3, %eax
	ret

/*
 * (x86) Returns the value of the %cr4 register.
 */
cr4_get:
	mov %cr4, %eax
	ret

/*
 * (x86) Sets the value of the %cr4 register.
 */
cr4_set:
	mov 4(%esp), %eax
	mov %eax, %cr4
	ret
