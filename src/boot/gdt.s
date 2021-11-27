.global GDT_KERNEL_CS
.global GDT_KERNEL_DS
.global GDT_USER_CS
.global GDT_USER_DS
.global GDT_TSS

.global GDT_PHYS_PTR
.global GDT_DESC_PHYS_PTR
.global GDT_VIRT_PTR
.global GDT_DESC_VIRT_PTR

.global gdt_start
.global gdt_kernel_code
.global gdt_kernel_data
.global gdt_user_code
.global gdt_user_data
.global gdt_tss
.global gdt

.global switch_protected
.global gdt_move

.extern gdt_copy

/*
 * Offsets into the GDT for each segment.
 */
.set GDT_KERNEL_CS, (gdt_kernel_code - gdt_start)
.set GDT_KERNEL_DS, (gdt_kernel_data - gdt_start)
.set GDT_USER_CS, (gdt_user_code - gdt_start)
.set GDT_USER_DS, (gdt_user_data - gdt_start)
.set GDT_TSS, (gdt_tss - gdt_start)

/*
 * Physical address to the GDT.
 */
.set GDT_PHYS_PTR,		0x800
/*
 * Physical address to the GDT descriptor.
 */
.set GDT_DESC_PHYS_PTR,	(GDT_PHYS_PTR + (gdt - gdt_start))
/*
 * Virtual address to the GDT.
 */
.set GDT_VIRT_PTR,		(0xc0000000 + GDT_PHYS_PTR)
/*
 * Virtual address to the GDT descriptor.
 */
.set GDT_DESC_VIRT_PTR,	(GDT_VIRT_PTR + (gdt - gdt_start))

.section .boot.text

/*
 * Switches the CPU to protected mode.
 */
switch_protected:
	cli

	call gdt_copy
	mov $GDT_DESC_PHYS_PTR, %eax
	movl $GDT_PHYS_PTR, 2(%eax)

	lgdt GDT_DESC_PHYS_PTR

	mov %cr0, %eax
	or $1, %al
	mov %eax, %cr0

	jmp $GDT_KERNEL_CS, $complete_flush
complete_flush:
	mov $GDT_KERNEL_DS, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	mov %ax, %ss
	ret

/*
 * Moves the GDT to the new virtual address after kernel relocation.
 */
gdt_move:
	mov $GDT_DESC_VIRT_PTR, %eax
	movl $GDT_VIRT_PTR, 2(%eax)

	lgdt GDT_DESC_VIRT_PTR

	ret



.section .boot.data

.align 8

/*
 * The beginning of the GDT.
 * Every segment covers the whole memory space.
 */
gdt_start:
gdt_null:
	.quad 0

/*
 * Segment for the kernel code.
 */
gdt_kernel_code:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10011010
	.byte 0b11001111
	.byte 0

/*
 * Segment for the kernel data.
 */
gdt_kernel_data:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10010010
	.byte 0b11001111
	.byte 0

/*
 * Segment for the user code.
 */
gdt_user_code:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b11111010
	.byte 0b11001111
	.byte 0

/*
 * Segment for the user data.
 */
gdt_user_data:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b11110010
	.byte 0b11001111
	.byte 0

/*
 * Reserved space for the Task State Segment.
 */
gdt_tss:
	.quad 0

/*
 * TLS GDT entries.
 */
gdt_tls:
	.quad 0
	.quad 0
	.quad 0

/*
 * The GDT descriptor.
 */
gdt:
	.word gdt - gdt_start - 1
	.long gdt_start
