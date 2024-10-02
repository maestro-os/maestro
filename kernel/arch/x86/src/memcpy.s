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

.global memcpy
.type memcpy, @function

memcpy:
	push esi
	push edi

	mov edi, [esp + 12]
	mov esi, [esp + 16]
	mov ecx, [esp + 20]

	mov eax, edi

	cmp ecx, 4
	jc loop
	test edi, 3
	jz loop

pad:
	movsb
	dec ecx
	test edi, 3
	jnz pad

loop:
	mov edx, ecx
	shr ecx, 2
	rep movsd
	and edx, 3
	jz end

remain:
	movsb
	dec edx
	jnz remain

end:
	pop edi
	pop esi
	ret
