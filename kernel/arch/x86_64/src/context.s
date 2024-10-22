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

.include "arch/x86_64/src/regs.s"

// Context switch functions

.global context_switch32
.global context_switch64
.global context_switch_kernel

.type context_switch32, @function
.type context_switch64, @function
.type context_switch_kernel, @function

context_switch32:
    # TODO
    ud2

context_switch64:
    # TODO
    ud2

context_switch_kernel:
    # TODO
    ud2

// System calls handler

.global syscall
.type syscall, @function

syscall:
    # TODO
    ud2
