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

.section .text

.global stack_switch_

.type stack_switch, @function

# Performs the stack switching for the given stack and closure to execute.
stack_switch_:
	push %ebp
	mov %esp, %ebp

	# Setting the new stack
	mov 8(%ebp), %esp # `stack` argument
	push 12(%ebp) # `s` argument
	push 16(%ebp) # `f` argument

	# Saving ebp and setting it to zero to prevent crashes when iterating on the callstack
	push %ebp
	xor %ebp, %ebp

	# Calling the given function
	push 8(%esp) # `s` argument
	call *8(%esp)
	add $4, %esp

	# Restoring ebp
	pop %ebp

	mov %ebp, %esp
	pop %ebp
	ret
