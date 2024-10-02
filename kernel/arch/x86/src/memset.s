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

.global memset
.type memset, @function

# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT
memset:
	mov ecx, [esp + 12]
	cmp ecx, 62
	ja 2f

	mov dl, [esp + 8]
	mov eax, [esp + 4]
	test ecx, ecx
	jz 1f

	mov dh, dl

	mov [eax], dl
	mov [eax + ecx - 1], dl
	cmp ecx, 2
	jbe 1f

	mov [eax + 1], dx
	mov [eax + ecx + -1-2], %dx
	cmp ecx, 6
	jbe 1f

	shl edx, 16
	mov dl, [esp + 8]
	mov dh, [esp + 8]

	mov [eax + 1+2], %edx
	mov [eax + ecx + -1-2-4], edx
	cmp ecx, 14
	jbe 1f

	mov [eax + 1+2+4], edx
	mov [eax + 1+2+4+4], edx
	mov [eax + ecx + -1-2-4-8], edx
	mov [eax + ecx + -1-2-4-4], edx
	cmp ecx, 30
	jbe 1f

	mov [eax + 1+2+4+8], edx
	mov [eax + 1+2+4+8+4], edx
	mov [eax + 1+2+4+8+8], edx
	mov [eax + 1+2+4+8+12], edx
	mov [eax + ecx + -1-2-4-8-16], edx
	mov [eax + ecx + -1-2-4-8-12], edx
	mov [eax + ecx + -1-2-4-8-8], edx
	mov [eax + ecx + -1-2-4-8-4], edx

1:
    ret

2:
	movzbl eax, [esp + 8]
	mov [esp + 12], edi
	imul eax, 0x1010101
	mov edi, [esp + 4]
	test edi, 15
	mov [edi + ecx - 4], eax
	jnz 2f

1:
    shr ecx, 2
	rep
	stosl
	mov eax, [esp + 4]
	mov edi, [esp + 12]
	ret
	
2:
    xor edx, edx
	sub edx, edi
	and edx, 15
	mov [edi], eax
	mov [edi + 4], eax
	mov [edi + 8], eax
	mov [edi + 12], eax
	sub ecx, edx
	add edi, edx
	jmp 1b
