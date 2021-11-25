/*
 * Context switching allows to stop the currently executed code, changing the state of the machine to another saved state.
 */

.global context_switch
.global context_switch_kernel

.extern end_of_interrupt

.section .text

/*
 * This function switches to a userspace context.
 */
context_switch:
	cli
	mov %esp, %ebp

	# Setting segment registers
	mov 8(%ebp), %eax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	# Setting registers, except %eax
	mov 4(%ebp), %eax
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi
	fstcw 40(%eax)
	stmxcsr 44(%eax)

	# Placing iret data on the stack
	# (Note: If set, the interrupt flag in eflags will enable the interruptions back after using `iret`)
	push 8(%ebp) # data segment selector
	push 4(%eax) # esp
	push 12(%eax) # eflags
	push 12(%ebp) # code segment selector
	push 8(%eax) # eip

	# Setting %eax
	push 16(%ebp)
	mov (%eax), %ebp
	mov 16(%eax), %eax

	add $4, %esp

	iret

/*
 * This function switches to a kernelspace context.
 */
context_switch_kernel:
	cli

	mov 4(%esp), %eax

	# Setting eflags without the interrupt flag
	mov 12(%eax), %ebx
	mov $512, %ecx
	not %ecx
	and %ecx, %ebx
	push %ebx
	popf

	# Setting registers
	mov (%eax), %ebp
	mov 4(%eax), %esp
	mov 8(%eax), %ebx
	movl %ebx, jmp_addr
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi
	fstcw 40(%eax)
	stmxcsr 44(%eax)
	mov 16(%eax), %eax

	# TODO FIXME: Writing to global memory is not thread-safe
	# Setting the interrupt flag and jumping to kernel code execution
	# (Note: These two instructions, if placed in this order are atomic on x86, meaning that an interrupt cannot happen in between)
	sti
	jmp *jmp_addr

.section .data

// A location in memory storing the pointer to jump to.
// This location has to be used to avoid using a register.
jmp_addr:
	.long 0
