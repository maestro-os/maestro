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
 * This file implements functions for the structure containing registers's states
 */

// The size in bytes of the structure storing the registers' states
.set REGS_SIZE, 560

/*
 * This macro stores the values of every registers after an interruption was triggered.
 *
 * Memory is allocated on the stack to store the values.
 *
 * The stack frame is used as a reference to place the register values.
 */
.macro GET_REGS n
	# Allocate space on the stack to store the registers
	sub $REGS_SIZE, %esp

	# Fill segments in the structure
	movl $0, 0x2c(%esp)
	mov %fs, 0x2c(%esp)
	movl $0, 0x28(%esp)
	mov %gs, 0x28(%esp)

	# Fill registers in the structure
	mov %edi, 0x24(%esp)
	mov %esi, 0x20(%esp)
	mov %edx, 0x1c(%esp)
	mov %ecx, 0x18(%esp)
	mov %ebx, 0x14(%esp)
	mov %eax, 0x10(%esp)

	# Save the fx state
	mov %esp, %eax
	add $0x30, %eax
	push %eax
	call save_fxstate
	add $4, %esp

	mov 12(%ebp), %eax
	mov %eax, 0xc(%esp) # eflags
	mov 4(%ebp), %eax
	mov %eax, 0x8(%esp) # eip

	cmpl $0x8, 8(%ebp)
	je ring0_\n
	jmp ring3_\n

# If the interruption was raised while executing on ring 0
ring0_\n:
	mov %ebp, %eax
	add $16, %eax
	mov %eax, 0x4(%esp) # esp
	jmp esp_end_\n

# If the interruption was raised while executing on ring 3
ring3_\n:
	mov 16(%ebp), %eax
	mov %eax, 0x4(%esp) # esp

esp_end_\n:
	mov (%ebp), %eax
	mov %eax, 0x0(%esp) # ebp
.endm



/*
 * This macro restores the registers' states and frees the space allocated by the function GET_REGS.
 */
.macro RESTORE_REGS
	# Restore the fx state
	mov %esp, %eax
	add $0x30, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	# Restore segments
	mov 0x2c(%esp), %fs
	mov 0x28(%esp), %gs

	# Restore registers
	mov 0x24(%esp), %edi
	mov 0x20(%esp), %esi
	mov 0x1c(%esp), %edx
	mov 0x18(%esp), %ecx
	mov 0x14(%esp), %ebx
	mov 0x10(%esp), %eax

	# Free the space allocated on the stack
	add $REGS_SIZE, %esp
.endm
