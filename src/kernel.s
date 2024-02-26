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

.global kernel_begin
.global kernel_end

.global kernel_loop_reset
.type kernel_loop_reset, @function

.section .text

/*
 * The kernel begin symbol, giving the pointer to the begin of the kernel image
 * in the virtual memory.
 */
kernel_begin:

/*
 * Resets the stack to the given value, then halts until an interruption is triggered.
 */
kernel_loop_reset:
	mov 4(%esp), %esp
	mov $0, %ebp
loop:
    sti
    hlt
	jmp loop

.section .bss

/*
 * The kernel end symbol, giving the pointer to the end of the kernel image in
 * the virtual memory.
 */
kernel_end:
