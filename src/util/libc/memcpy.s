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

.global memcpy
.type memcpy, @function

memcpy:
	push %esi
	push %edi

	mov 12(%esp), %edi
	mov 16(%esp), %esi
	mov 20(%esp), %ecx

	mov %edi, %eax

	cmp $4, %ecx
	jc loop
	test $3, %edi
	jz loop

pad:
	movsb
	dec %ecx
	test $3, %edi
	jnz pad

loop:
	mov %ecx, %edx
	shr $2, %ecx
	rep movsl
	and $3, %edx
	jz end

remain:
	movsb
	dec %edx
	jnz remain

end:
	pop %edi
	pop %esi
	ret
