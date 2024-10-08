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

.intel_syntax noprefix

/*
 * Register save/restore macros.
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
.macro GET_REGS
	# Allocate space on the stack to store the registers
	sub esp, REGS_SIZE

	# Fill segments in the structure
	mov dword ptr [esp + 40], 0
	mov [esp + 40], gs
	mov dword ptr [esp + 44], 0
	mov [esp + 44], fs

	# Fill registers in the structure
	mov [esp + 36], edi
	mov [esp + 32], esi
	mov [esp + 28], edx
	mov [esp + 24], ecx
	mov [esp + 20], ebx
	mov [esp + 16], eax

	mov eax, [ebp + 12]
	mov [esp + 12], eax # eflags
	mov eax, [ebp + 4]
	mov [esp + 8], eax # eip

    # Prevent userspace from breaking kernel
	cld

	# Save the fx state
	mov eax, esp
	add eax, 0x30
	push eax
	call save_fxstate
	add esp, 4

	cmp dword ptr [ebp + 8], 0x8
	je 0f
	jmp 1f

# If the interruption was raised while executing on ring 0
0:
	mov eax, ebp
	add eax, 16
	mov [esp + 4], eax # esp
	jmp 2f

# If the interruption was raised while executing on ring 3
1:
	mov eax, [ebp + 16]
	mov [esp + 4], eax # esp

2:
	mov eax, [ebp]
	mov [esp], eax # ebp
.endm



/*
 * This macro restores the registers' states and frees the space allocated by the function GET_REGS.
 */
.macro RESTORE_REGS
	# Restore the fx state
	mov eax, esp
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	# Restore segments
	mov gs, [esp + 40]
	mov fs, [esp + 44]

	# Restore registers
	mov edi, [esp + 36]
	mov esi, [esp + 32]
	mov edx, [esp + 28]
	mov ecx, [esp + 24]
	mov ebx, [esp + 20]
	mov eax, [esp + 16]

	# Free the space allocated on the stack
	add esp, REGS_SIZE
.endm
