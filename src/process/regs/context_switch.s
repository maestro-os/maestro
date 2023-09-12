/*
 * Context switching allows to stop the currently executed code, changing the state of the machine to another saved state.
 */

.section .text

.global context_switch
.global context_switch_kernel

.type context_switch, @function
.type context_switch_kernel, @function

.extern end_of_interrupt

/*
 * This function switches to a userspace context.
 */
context_switch:
	cli

	# Set segment registers
	mov 8(%esp), %eax
	mov %ax, %ds
	mov %ax, %es

	# Restore the fx state
	mov 4(%esp), %eax
	add $0x30, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	# Set registers, except %eax
	mov 4(%esp), %eax
	mov 0x0(%eax), %ebp
	mov 0x14(%eax), %ebx
	mov 0x18(%eax), %ecx
	mov 0x1c(%eax), %edx
	mov 0x20(%eax), %esi
	mov 0x24(%eax), %edi
	mov 0x28(%eax), %gs
	mov 0x2c(%eax), %fs

	# Place iret data on the stack
	# (Note: If set, the interrupt flag in eflags will enable the interruptions back after using `iret`)
	push 8(%esp) # data segment selector
	push 0x4(%eax) # esp
	push 0xc(%eax) # eflags
	push 24(%esp) # code segment selector
	push 0x8(%eax) # eip

	# Set %eax
	mov 0x10(%eax), %eax

	iret

/*
 * This function switches to a kernelspace context.
 */
context_switch_kernel:
	cli

	# Restore the fx state
	mov 4(%esp), %eax
	add $0x30, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	mov 4(%esp), %eax

	# Set eflags without the interrupt flag
	mov 12(%eax), %ebx
	mov $512, %ecx
	not %ecx
	and %ecx, %ebx
	push %ebx
	popf

	# Set registers
	mov 0x0(%eax), %ebp
	mov 0x4(%eax), %esp
	push 0x8(%eax) # eip
	mov 0x14(%eax), %ebx
	mov 0x18(%eax), %ecx
	mov 0x1c(%eax), %edx
	mov 0x20(%eax), %esi
	mov 0x24(%eax), %edi
	mov 0x28(%eax), %gs
	mov 0x2c(%eax), %fs
	mov 0x10(%eax), %eax

	# Set the interrupt flag and jumping to kernel code execution
	# (Note: These two instructions, if placed in this order are atomic on x86, meaning that an interrupt cannot happen in between)
	sti
	ret
