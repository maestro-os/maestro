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

.global __kernel_vsyscall
.global __kernel_rt_sigreturn
.global __kernel_sigreturn
.global __vdso_clock_gettime
.global __vdso_gettimeofday
.global __vdso_time

__kernel_vsyscall:
	int $0x80
	ret

__kernel_rt_sigreturn:
	# TODO
	ud2

__kernel_sigreturn:
	# TODO
	ud2

__vdso_clock_gettime:
	# TODO
	ud2

__vdso_gettimeofday:
	# TODO
	ud2

__vdso_time:
	# TODO
	ud2
