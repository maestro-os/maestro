/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

/*
 * This file implements the function that handles the system calls.
 */

.include "src/process/regs/regs.s"

.section .text

.global syscall
.type syscall, @function

/*
 * The function handling system calls.
 */
syscall:
	cli
	push %ebp
	mov %esp, %ebp

	# Storing registers state
GET_REGS

	# Setting data segment
	mov $GDT_KERNEL_DS, %ax
	mov %ax, %ds
	mov %ax, %es

	# Calling the system call handler
	push %esp
	sti
	call syscall_handler
	cli
	add $4, %esp

	# Restoring data segment
	xor %ebx, %ebx
	mov $GDT_USER_DS, %bx
	or $3, %bx
	mov %bx, %ds
	mov %bx, %es

RESTORE_REGS

	# Restoring the context
	mov %ebp, %esp
	pop %ebp
	iret
