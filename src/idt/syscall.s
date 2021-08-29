/*
 * This file implements the function that handles the system calls.
 */

.text

.global syscall

/*
 * The function handling system calls.
 */
syscall:
	cli
	push %ebp
	mov %esp, %ebp

	# Storing registers state
	push %edi
	push %esi
	push %edx
	push %ecx
	push %ebx
	push %eax
	push 12(%ebp) # eflags
	push 4(%ebp) # eip
	push 16(%ebp) # esp
	push (%ebp) # ebp

	# Setting segments
	mov $GDT_KERNEL_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	# Calling the system call handler
	push %esp
	sti
	call syscall_handler
	cli
	add $4, %esp

	# Restoring segments
	xor %ebx, %ebx
	mov $GDT_USER_DATA_OFFSET, %bx
	or $3, %bx
	mov %bx, %ds
	mov %bx, %es
	mov %bx, %fs
	mov %bx, %gs

	# Restoring registers state
	add $16, %esp
	pop %eax
	pop %ebx
	pop %ecx
	pop %edx
	pop %esi
	pop %edi

	mov %ebp, %esp
	pop %ebp
	sti
	iret
