.global tss_gdt_entry
.global tss_flush

.extern switch_protected

tss_gdt_entry:
	movl $gdt_tss, %eax

	ret

tss_flush:
	movw $GDT_TSS_OFFSET, %ax
	ltr %ax
	lgdt gdt

	ret
