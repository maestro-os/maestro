/*
 * This file is an extention to the TSS Rust module.
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
	movw $GDT_TSS, %ax
	ltr %ax
	lgdt GDT_DESC_VIRT_PTR

	ret

.section .data

// The size of the TSS structure in bytes.
.set TSS_SIZE, 104

// The location of the TSS.
tss:
	.skip TSS_SIZE
