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

#include "libc.h"

void *memcpy(void *dest, const void *src, size_t n);

// TODO Optimize
void *memmove(void *dest, const void *src, size_t n)
{
       void *begin = dest;
       size_t i = 0;

       if (dest < src)
               return memcpy(dest, src, n);
       while (i < n)
       {
               ((char *) dest)[n - i - 1] = ((char *) src)[n - i - 1];
               ++i;
       }
       return begin;
}
