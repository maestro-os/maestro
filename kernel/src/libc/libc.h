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

// C functions utils

#ifndef LIBC_H
# define LIBC_H

// Gives the offset of the pointer `ptr` relative to its down-aligned
// counterpart.
# define ALIGN_MASK(ptr, n)	((intptr_t) (ptr) & ((n) - 1))

// Tells whether the pointer `ptr` is aligned on boundary `n`.
//
// If `n` is zero, the behaviour is undefined.
# define IS_ALIGNED(ptr, n)	(ALIGN_MASK(ptr, n) == 0)

// Aligns down the given memory pointer `ptr` to the boundary `n`.
//
// If `n` is zero, the behaviour is undefined.
# define DOWN_ALIGN(ptr, n)\
	(typeof(ptr)) ((intptr_t) (ptr) & ~((intptr_t) ((n) - 1)))

#endif
