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

.include "arch/x86/src/regs.s"

// Context switch functions

.global context_switch32
.global context_switch_kernel

.type context_switch32, @function
.type context_switch_kernel, @function

context_switch32:
	# Restore the fx state
	mov eax, [esp + 4]
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	# Set segment registers
	mov ax, (32 | 3)
	mov ds, ax
	mov es, ax

	# Set registers, except eax
	mov eax, [esp + 4]
	mov ebp, [eax]
	mov ebx, [eax + 20]
	mov ecx, [eax + 24]
	mov edx, [eax + 28]
	mov esi, [eax + 32]
	mov edi, [eax + 36]
	mov gs, [eax + 40]
	mov fs, [eax + 44]

	# Place iret data on the stack
	push (32 | 3) # data segment selector
	push [eax + 4] # esp
	push [eax + 12] # eflags
	push (24 | 3) # code segment selector
	push [esp + 24] # eip

	# Set eax
	mov eax, [eax + 16]

	iretd

context_switch_kernel:
	# Restore the fx state
	mov eax, [esp + 4]
	add eax, 0x30
	push eax
	call restore_fxstate
	add esp, 4

	mov eax, [esp + 4]

	# Set eflags without the interrupt flag
	mov ebx, [eax + 12]
	mov ecx, 512
	not ecx
	and ebx, ecx
	push ebx
	popfd

	# Set registers
	mov ebp, [eax]
	mov esp, [eax + 4]
	push [eax + 8] # eip
	mov [eax + 20], ebx
	mov [eax + 24], ecx
	mov [eax + 28], edx
	mov [eax + 32], esi
	mov [eax + 36], edi
	mov [eax + 40], gs
	mov [eax + 44], fs
	mov [eax + 16], eax

	# Set the interrupt flag and jumping to kernel code execution
	# (Note: These two instructions, if placed in this order are atomic on x86, meaning that an interrupt cannot happen in between)
	sti
