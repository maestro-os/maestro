.global tss_flush

/*
 * x86. Updates the TSS into the GDT.
 */
tss_flush:
	movw $GDT_TSS_OFFSET, %ax
	ltr %ax
	lgdt GDT_DESC_VIRT_PTR

	ret
