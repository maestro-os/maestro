.global GDT_KERNEL_CODE_OFFSET
.global GDT_KERNEL_DATA_OFFSET
.global GDT_USER_CODE_OFFSET
.global GDT_USER_DATA_OFFSET
.global GDT_TSS_OFFSET
.global gdt
.global gdt_tss

.global switch_protected

/*
 * Offsets into the GDT for each segment.
 */
.set GDT_KERNEL_CODE_OFFSET, (gdt_kernel_code - gdt_start)
.set GDT_KERNEL_DATA_OFFSET, (gdt_kernel_data - gdt_start)
.set GDT_USER_CODE_OFFSET, (gdt_user_code - gdt_start)
.set GDT_USER_DATA_OFFSET, (gdt_user_data - gdt_start)
.set GDT_TSS_OFFSET, (gdt_tss - gdt_start)

.section .boot.text

/*
 * Switches the CPU to protected mode.
 */
switch_protected:
	cli
	lgdt gdt
	mov %cr0, %eax
	or $1, %al
	mov %eax, %cr0

	jmp $0x8, $complete_flush
complete_flush:
	mov $0x10, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs
	mov %ax, %ss
	ret



.section .gdt

. = (0x800 - gdt)

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

gdt:
	.word gdt - gdt_start - 1
	.long gdt_start
