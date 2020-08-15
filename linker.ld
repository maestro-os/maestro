SECTIONS
{
/*
 * Booting stub sections
 */
	. = 0x100000;

	.boot.text ALIGN(4K) :
	{
		*(.boot.text)
	}

	.boot.data ALIGN(4K) :
	{
		*(.boot.data)
	}

	.boot.stack ALIGN(4K) :
	{
		*(.boot.stack)
	}

/*
 * Kernel sections
 */
	. = 0xc0100000;

	.text ALIGN(4K) : AT (ADDR (.text) - 0xc0000000 + 0x100000)
	{
/*
 * Symbol at the beginning of the kernel image.
 */
		kernel_begin = .;

		*(.text)
	}

	.rodata ALIGN(4K) : AT (ADDR (.rodata) - 0xc0000000 + 0x100000)
	{
		*(.rodata)
	}

	.data ALIGN(4K) : AT (ADDR (.data) - 0xc0000000 + 0x100000)
	{
		*(.data)
	}

	.bss ALIGN(4K) : AT (ADDR (.bss) - 0xc0000000 + 0x100000)
	{
		*(COMMON)
		*(.bss)

/*
 * Symbol at the end of the kernel image
 */
		kernel_end = .;
	}
}
