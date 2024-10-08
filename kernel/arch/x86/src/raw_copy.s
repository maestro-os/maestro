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

// Copy from/to userspace

.intel_syntax noprefix

.section .text

.global raw_copy
.global copy_fault

// TODO can be optimized
raw_copy:
	push esi
	push edi

	mov esi, 12[esp]
	mov edi, 16[esp]
	mov ecx, 20[esp]

	rep movsb

	pop edi
	pop esi
	mov eax, 1
	ret

copy_fault:
	pop edi
	pop esi
	xor eax, eax
	ret
