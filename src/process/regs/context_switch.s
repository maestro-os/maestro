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

	# Restoring the fx state
	mov 4(%ebp), %eax
	add $0x28, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	# Setting registers, except %eax
	mov 4(%ebp), %eax
	mov 0x14(%eax), %ebx
	mov 0x18(%eax), %ecx
	mov 0x1c(%eax), %edx
	mov 0x20(%eax), %esi
	mov 0x24(%eax), %edi

	# Placing iret data on the stack
	# (Note: If set, the interrupt flag in eflags will enable the interruptions back after using `iret`)
	push 8(%ebp) # data segment selector
	push 0x4(%eax) # esp
	push 0xc(%eax) # eflags
	push 12(%ebp) # code segment selector
	push 0x8(%eax) # eip

	# Setting %eax
	mov 0x0(%eax), %ebp
	mov 0x10(%eax), %eax

	iret

/*
 * This function switches to a kernelspace context.
 */
context_switch_kernel:
	cli

	# Restoring the fx state
	mov 4(%ebp), %eax
	add $0x28, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	mov 4(%esp), %eax

	# Setting eflags without the interrupt flag
	mov 12(%eax), %ebx
	mov $512, %ecx
	not %ecx
	and %ecx, %ebx
	push %ebx
	popf

	# Setting registers
	mov 0x0(%eax), %ebp
	mov 0x4(%eax), %esp
	push 0x8(%eax) # eip
	mov 0x14(%eax), %ebx
	mov 0x18(%eax), %ecx
	mov 0x1c(%eax), %edx
	mov 0x20(%eax), %esi
	mov 0x24(%eax), %edi
	mov 0x10(%eax), %eax

	# Setting the interrupt flag and jumping to kernel code execution
	# (Note: These two instructions, if placed in this order are atomic on x86, meaning that an interrupt cannot happen in between)
	sti
	ret
