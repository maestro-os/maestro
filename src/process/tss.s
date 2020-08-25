.global tss_gdt_entry
.global tss_flush

.extern switch_protected

/*
 * x86. Returns a pointer to the Task State Segment entry into the Global
 * Descriptor Table.
 */
tss_gdt_entry:
	movl $gdt_tss, %eax

	ret

/*
 * x86. Updates the TSS into the GDT.
 */
tss_flush:
	movw $GDT_TSS_OFFSET, %ax
	ltr %ax
	lgdt GDT_VIRT_PTR

	ret
