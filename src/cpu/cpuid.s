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

/*
 * File implementing CPUID-related features.
 */

.global cpuid_has_sse
.global get_hwcap

.type cpuid_has_sse, @function
.type get_hwcap, @function

.section .text

cpuid_has_sse:
	push %ebx

	mov $0x1, %eax
	cpuid
	shr $25, %edx
	and $0x1, %edx
	mov %edx, %eax

	pop %ebx
	ret

get_hwcap:
	push %ebx

	mov $0x1, %eax
	cpuid
	mov %edx, %eax

	pop %ebx
	ret
