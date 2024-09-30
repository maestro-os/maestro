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

.global a20_handle

.type a20_handle, @function
.type a20_check, @function
.type a20_wait_read, @function
.type a20_wait_write, @function

.section .boot.text, "ax"

/*
 * Ensures that the A20 line is enabled.
 */
a20_handle:
	call a20_check
	test $0, %eax
	je a20_handle_
	ret
a20_handle_:
	call a20_enable
	ret

/*
 * Checks whether the a20 line is enabled or not.
 */
a20_check:
	pusha
	mov $0x888888, %edi
	mov $0x088888, %esi
	mov %edi, (%edi)
	mov %esi, (%esi)
	cmpsl
	popa
	jne a20_enabled
	xor %eax, %eax
	ret
a20_enabled:
	mov $1, %eax
	ret

/*
 * Enables the a20 line using the PS2 controller.
 * Note: Interrupts must be disabled for this function.
 */
a20_enable:
	pushf
	cli

	call a20_wait_write
	mov $0xad, %al
	outb %al, $0x64

	call a20_wait_write
	mov $0xd0, %al
	outb %al, $0x64

	call a20_wait_read
	inb $0x60, %al
	push %eax

	call a20_wait_write
	mov $0xd1, %al
	outb %al, $0x64

	pop %eax
	or $2, %al
	outb %al, $0x60

	call a20_wait_write

	popf
	ret

/*
 * Waits for the PS2 controller to be available for reading.
 */
a20_wait_read:
	in $0x64, %al
	test $1, %al
	jz a20_wait_read
	ret

/*
 * Waits for the PS2 controller to be available for writing.
 */
a20_wait_write:
	in $0x64, %al
	test $2, %al
	jnz a20_wait_write
	ret
