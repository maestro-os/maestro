/*
 * This file is the linker script for the x86 architecture.
 *
 * The kernel image is split into two parts:
 * - The booting part, in lower memory (sections beginning with `.boot`)
 * - The kernel part, in higher memory, starting at 0xc0000000
 *
 * Before really booting the kernel, the memory must be remapped so that the kernel image is
 * effectively relocated to higher memory.
 * After running the kernel code, the booting code isn't useful anymore.
 *
 * Sections need to be aligned on the page boundary to be protected against writing (for those
 * where it applies).
 */
SECTIONS
{
	ENTRY(multiboot_entry)

/*
 * Booting stub sections
 */
	. = 0x100000;

	.boot.text BLOCK(4K) : ALIGN(4K)
	{
		*(.boot.text)
	}

	.boot.stack BLOCK(4K) : ALIGN(4K)
	{
		*(.boot.stack)
	}

	.boot.data BLOCK(4K) : ALIGN(4K)
	{
		*(.boot.data)
	}

/*
 * Kernel sections
 */
	. = 0xc0200000;

	.text BLOCK(4K) : AT (ADDR (.text) - 0xc0000000) ALIGN(4K)
	{
/*
 * Symbol at the beginning of the kernel image.
 */
		kernel_begin = .;

		*(.text*)
	}

	.rodata BLOCK(4K) : AT (ADDR (.rodata) - 0xc0000000) ALIGN(4K)
	{
		*(.rodata*)
	}

	.data BLOCK(4K) : AT (ADDR (.data) - 0xc0000000) ALIGN(4K)
	{
		*(.data*)
	}

	.bss BLOCK(4K) : AT (ADDR (.bss) - 0xc0000000) ALIGN(4K)
	{
		*(COMMON)
		*(.bss*)

/*
 * Symbol at the end of the kernel image
 */
		kernel_end = .;
	}
}
