.set MULTIBOOT_MAGIC,			0xe85250d6
.set MULTIBOOT_ARCHITECTURE,	0
.set HEADER_LENGTH,				(header_end - header)
.set CHECKSUM,					-(MULTIBOOT_MAGIC + MULTIBOOT_ARCHITECTURE + HEADER_LENGTH)

.set MULTIBOOT_HEADER_TAG_END,					0
.set MULTIBOOT_HEADER_TAG_INFORMATION_REQUEST,	1
.set MULTIBOOT_HEADER_TAG_ADDRESS,				2
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS,		3
.set MULTIBOOT_HEADER_TAG_CONSOLE_FLAGS,		4
.set MULTIBOOT_HEADER_TAG_FRAMEBUFFER,			5
.set MULTIBOOT_HEADER_TAG_MODULE_ALIGN,			6
.set MULTIBOOT_HEADER_TAG_EFI_BS,				7
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI32,	8
.set MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI64,	9
.set MULTIBOOT_HEADER_TAG_RELOCATABLE,			10

.global switch_protected

.global GDT_KERNEL_CODE_OFFSET
.global GDT_KERNEL_USER_OFFSET
.global GDT_USER_CODE_OFFSET
.global GDT_USER_DATA_OFFSET
.global GDT_TSS_OFFSET
.global gdt

.set GDT_KERNEL_CODE_OFFSET, (gdt_kernel_code - gdt_start)
.set GDT_KERNEL_DATA_OFFSET, (gdt_kernel_data - gdt_start)
.set GDT_USER_CODE_OFFSET, (gdt_user_code - gdt_start)
.set GDT_USER_DATA_OFFSET, (gdt_user_data - gdt_start)
.set GDT_TSS_OFFSET, (gdt_tss - gdt_start)

.set STACK_SIZE,	16384

.section .text

.align 8
header:
	.long MULTIBOOT_MAGIC
	.long MULTIBOOT_ARCHITECTURE
	.long HEADER_LENGTH
	.long CHECKSUM

.align 8
entry_address_tag:
	.short MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS
	.short 1
	.long (entry_address_tag_end - entry_address_tag)
	.long multiboot_entry
entry_address_tag_end:

.align 8
	.short MULTIBOOT_HEADER_TAG_END
	.short 0
	.long 8
header_end:

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

kernel_init:
	call switch_protected
	ret

.global kernel_loop
.global kernel_halt

kernel_loop:
	sti
	hlt
	jmp kernel_loop

kernel_halt:
	cli
	hlt
	jmp kernel_halt

multiboot_entry:
	mov $stack_top, %esp

	pushl $0
	popf

	push %eax
	push %ebx
	call kernel_init
	call _init
	pop %ebx
	pop %eax

	pushl $kernel_end
	push %ebx
	push %eax
	call kernel_main

	call kernel_halt
	call _fini

.section .data

.global gdt_tss

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

.section .bss

.align 8

stack_bottom:
	.skip STACK_SIZE
stack_top:

.global kernel_end

kernel_end:
