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

.set GDT_KERNEL_CODE_OFFSET, (gdt_kernel_code - gdt_start)
.set GDT_KERNEL_DATA_OFFSET, (gdt_kernel_data - gdt_start)
.set GDT_USER_CODE_OFFSET, (gdt_user_code - gdt_start)
.set GDT_USER_DATA_OFFSET, (gdt_user_data - gdt_start)
.set GDT_TSS_OFFSET, (gdt_tss - gdt_start)

.set STACK_SIZE,	32768

.section .text

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

kernel_wait:
	sti
	hlt
	ret

kernel_loop:
	sti
	hlt
	jmp kernel_loop

kernel_halt:
	cli
	hlt
	jmp kernel_halt

.section .data

.align 8

gdt_start:
gdt_null:
	.quad 0

gdt_kernel_code:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10011010
	.byte 0b11001111
	.byte 0

gdt_kernel_data:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10010010
	.byte 0b11001111
	.byte 0

gdt_user_code:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b11111010
	.byte 0b11001111
	.byte 0

gdt_user_data:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b11110010
	.byte 0b11001111
	.byte 0

gdt_tss:
	.quad 0

gdt:
	.word gdt - gdt_start - 1
	.long gdt_start

.section .stack[write]

.align 8

stack_bottom:
	.skip STACK_SIZE
stack_top:
	.skip STACK_SIZE
switch_stack:

.section .bss

kernel_end:
