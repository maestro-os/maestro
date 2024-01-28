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

#include <stdint.h>
#include <stddef.h>

#define LOW ((size_t) -1 / 0xff)
#define HIGH (LOW * 0x80)
#define ZERO(w) (((w) - LOW) & (~(w) & HIGH))

size_t strlen(const char *s)
{
	const char *n = s;

    // Align
    for (; (uintptr_t) n % sizeof(size_t); ++n) if (!*n) return n - s;
    // Check word-by-word
    const size_t *word = (size_t *) n;
    for (; !ZERO(*word); ++word);
    n = (const char *) word;
    // Count remaining
    for (; *n; ++n)
        ;
	return n - s;
}
