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
 * The linker script for the x86_64 architecture.
 *
 * For more information about the organization of this particular file, check
 * the documentation in the linker script for the x86 architecture.
 */

ENTRY(multiboot_entry)

SECTIONS
{
	. = 0x100000;

	.boot.text : ALIGN(4K)
	{
		*(.boot.text)
	}

	.boot.stack : ALIGN(4K)
	{
		*(.boot.stack)
	}

	. = 0xc0200000;

	.text : AT (ADDR (.text) - 0xc0000000) ALIGN(4K)
	{
		*(.text*)
	}

	.rodata : AT (ADDR (.rodata) - 0xc0000000) ALIGN(4K)
	{
		*(.rodata*)
	}

	.user : AT (ADDR (.user) - 0xc0000000) ALIGN(4K)
	{
	    *(.user*)
	}

	.bss : AT (ADDR (.bss) - 0xc0000000) ALIGN(4K)
	{
		*(COMMON)
		*(.bss*)
	}

	.data : AT (ADDR (.data) - 0xc0000000) ALIGN(4K)
	{
		*(.data*)
	}
}
