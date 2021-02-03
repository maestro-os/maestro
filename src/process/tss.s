/*
 * TODO doc
 */

.global tss_get
.global tss_flush

.section .text

/*
 * x86. Returns the pointer to the TSS.
 */
tss_get:
	mov $tss, %eax
	ret

/*
 * x86. Updates the TSS into the GDT.
 */
tss_flush:
	movw $GDT_TSS_OFFSET, %ax
	ltr %ax
	lgdt GDT_DESC_VIRT_PTR

	ret

.section .data

// TODO doc
.set TSS_SIZE, 104

tss:
	.skip TSS_SIZE
