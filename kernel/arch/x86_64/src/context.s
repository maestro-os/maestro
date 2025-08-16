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
.section .text

.macro ERROR id
.global error\id
.type error\id, @function

error\id:
	push 0 # code (absent)
	push \id
	jmp int_common
.endm

.macro ERROR_CODE id
.global error\id
.type error\id, @function

error\id:
	push \id
	jmp int_common
.endm

.macro IRQ id
.global irq\id
.type irq\id, @function

irq\id:
	push 0 # code (absent)
	push (0x20 + \id)
	jmp int_common
.endm

ERROR 0
ERROR 1
ERROR 2
ERROR 3
ERROR 4
ERROR 5
ERROR 6
ERROR 7
ERROR_CODE 8
ERROR 9
ERROR_CODE 10
ERROR_CODE 11
ERROR_CODE 12
ERROR_CODE 13
ERROR_CODE 14
ERROR 15
ERROR 16
ERROR_CODE 17
ERROR 18
ERROR 19
ERROR 20
ERROR 21
ERROR 22
ERROR 23
ERROR 24
ERROR 25
ERROR 26
ERROR 27
ERROR 28
ERROR 29
ERROR_CODE 30
ERROR 31

IRQ 0
IRQ 1
IRQ 2
IRQ 3
IRQ 4
IRQ 5
IRQ 6
IRQ 7
IRQ 8
IRQ 9
IRQ 10
IRQ 11
IRQ 12
IRQ 13
IRQ 14
IRQ 15

.macro STORE_REGS
    push fs
    push gs
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax
.endm

.macro LOAD_REGS
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    # discard fs and gs
    # This is necessary since setting either to null clears the associated hidden base register on Intel processors
    add rsp, 16
.endm

.global idt_ignore
.global init_ctx
.global syscall_int
.global syscall
.global idle_task
.type init_ctx, @function
.type syscall_int, @function
.type syscall, @function
.type idle_task, @function

int_common:
    # Handle swapgs nesting
    cmp qword ptr [rsp + 24], 8
    je 1f
    swapgs
1:

STORE_REGS
	cld
	mov rdi, rsp
	call interrupt_handler
LOAD_REGS

# Handle swapgs nesting
    cmp qword ptr [rsp + 24], 8
    je 1f
    swapgs
1:

	add rsp, 16
	iretq

idt_ignore:
    iretq

init_ctx:
	# Set user data segment
	mov ax, 0x23
	mov es, ax
	mov ds, ax
	mov rsp, rdi
	LOAD_REGS
	add rsp, 16
	swapgs
	iretq

syscall_int:
    # `swapgs` is safe here because this entry may be called only from userspace
    swapgs
	cld
	push 0 # code (absent)
	push 0 # interrupt ID (absent)
STORE_REGS
	mov rdi, rsp
	call syscall_handler
LOAD_REGS
	add rsp, 16
    swapgs
	iretq

syscall:
    # Switch to kernelspace stack
    swapgs
    mov [gs:0x8], rsp
    mov rsp, [gs:0x0]

    sti

    # Push artificial iret frame
    push 0x23
    push [gs:0x8]
    push r11
    push 0x2b
    push rcx

    push 0 # code (absent)
    push 0 # interrupt ID (absent)

STORE_REGS

	mov rdi, rsp
	call syscall_handler

    # Cleanup
LOAD_REGS
	cli
	mov rsp, [rsp + 0x28]
	swapgs
    sysretq

idle_task:
    sti
    hlt
    jmp idle_task
