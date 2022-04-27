.global stack_switch_

.extern stack_switch_in

.section .text

# Performs the stack switching for the given stack and closure to execute.
stack_switch_:
	push %ebp
	mov %esp, %ebp

	mov 8(%ebp), %esp # `stack` argument
	push 12(%ebp) # `s` argument
	call 16(%ebp)

	mov %ebp, %esp
	pop %ebp
	ret
