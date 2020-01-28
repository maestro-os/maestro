.global GDT_KERNEL_CODE_OFFSET
.global GDT_KERNEL_DATA_OFFSET
.global GDT_USER_CODE_OFFSET
.global GDT_USER_DATA_OFFSET
.global GDT_TSS_OFFSET
.global gdt
.global gdt_tss

.global switch_protected
.global kernel_wait
.global kernel_loop
.global kernel_halt

.global stack_top
.global switch_stack
.global kernel_end

/*
 * Offsets into the GDT for each segment.
 */
.set GDT_KERNEL_CODE_OFFSET, (gdt_kernel_code - gdt_start)
.set GDT_KERNEL_DATA_OFFSET, (gdt_kernel_data - gdt_start)
.set GDT_USER_CODE_OFFSET, (gdt_user_code - gdt_start)
.set GDT_USER_DATA_OFFSET, (gdt_user_data - gdt_start)
.set GDT_TSS_OFFSET, (gdt_tss - gdt_start)

/*
 * The size of the kernel stack.
 */
.set STACK_SIZE,	32768

.section .text

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

/*
 * Makes the kernel wait for an interrupt.
 */
kernel_wait:
	sti
	hlt
	ret

/*
 * Enters the kernel loop, process every interrupt indefinitely.
 */
kernel_loop:
	sti
	hlt
	jmp kernel_loop

/*
 * Halts the kernel forever.
 */
kernel_halt:
	cli
	hlt
	jmp kernel_halt

.section .data

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

gdt:
	.word gdt - gdt_start - 1
	.long gdt_start

.section .stack, "w"

.align 8

/*
 * The kernel stack.
 */
stack_bottom:
	.skip STACK_SIZE
stack_top:
	.skip STACK_SIZE
switch_stack:

.section .bss

/*
 * The kernel end symbol.
 */
kernel_end:
