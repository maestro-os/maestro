.section .text

.global stack_switch_

.type stack_switch, @function

# Performs the stack switching for the given stack and closure to execute.
stack_switch_:
	push %ebp
	mov %esp, %ebp

	# Setting the new stack
	mov 8(%ebp), %esp # `stack` argument
	push 12(%ebp) # `s` argument
	push 16(%ebp) # `f` argument

	# Saving ebp and setting it to zero to prevent crashes when iterating on the callstack
	push %ebp
	xor %ebp, %ebp

	# Calling the given function
	push 8(%esp) # `s` argument
	call *8(%esp)
	add $4, %esp

	# Restoring ebp
	pop %ebp

	mov %ebp, %esp
	pop %ebp
	ret
