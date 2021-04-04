/*
 * Context switching allows to stop the currently executed code, changing the state of the machine to another saved state.
 */

.global context_switch
.global context_switch_kernel

.extern end_of_interrupt

.section .text

/*
 * This function switches to a userspace context.
 * TODO document arguments
 */
context_switch:
	cli
	mov %esp, %ebp
	#mov $boot_stack_top, %esp # TODO remove

	mov 8(%ebp), %eax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	mov 4(%ebp), %eax
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi

	push 8(%ebp)
	push 4(%eax)
	pushf
	orl $0x200, (%esp)
	push 12(%ebp)
	push 8(%eax)

	push 16(%ebp)
	mov (%eax), %ebp
	mov 16(%eax), %eax

	pusha
	push $0x0
	call end_of_interrupt
	add $4, %esp
	popa
	add $4, %esp

	iret

/*
 * This function switches to a kernelspace context.
 * TODO document arguments
 */
context_switch_kernel:
	cli

	push $0x0
	call end_of_interrupt
	add $4, %esp

	mov 4(%esp), %eax
	mov 12(%eax), %ebx
	mov $512, %ecx
	not %ecx
	and %ecx, %ebx
	push %ebx
	popf
	mov (%eax), %ebp
	mov 4(%eax), %esp
	mov 8(%eax), %ebx
	movl %ebx, jmp_addr
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi
	mov 16(%eax), %eax

	sti
	jmp *jmp_addr

.section .data

// TODO doc (remove?)
jmp_addr:
	.long 0
