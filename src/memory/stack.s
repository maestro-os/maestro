/*
 * This file implements the stack switching function.
 */

.global stack_switch_

.section .text

# Performs the stack switching for the given stack and closure to execute.
stack_switch_:
	push %ebp
	mov %esp, %ebp

	mov 12(%ebp), %eax
	mov 16(%ebp), %esp
	push 8(%ebp)
	call *%eax
	add $4, %esp

	mov %ebp, %esp
	pop %ebp
	ret
