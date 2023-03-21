SECTIONS
{
	ENTRY(__kernel_vsyscall)

	. = 0x1000;

	.text BLOCK(4K) : ALIGN(4K)
	{
		*(.text)
	}
}
