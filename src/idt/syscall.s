/*
 * This file implements the function that handles the system calls.
 */

.include "src/process/regs/regs.s"

.global syscall

.section .text

/*
 * The function handling system calls.
 */
syscall:
	cli
	push %ebp
	mov %esp, %ebp

	# Storing registers state
GET_REGS

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
END_INTERRUPT
