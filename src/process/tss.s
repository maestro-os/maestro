.global tss_gdt_entry
.global tss_flush

tss_gdt_entry:
	mov gdt_tss, %eax

	ret

tss_flush:
	mov TSS_OFFSET, %ax
	ltr %ax

	ret
