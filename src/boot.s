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

.set STACK_SIZE,	16384

.section .text
.global start, _start

start:
_start:
	jmp multiboot_entry

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

#.align 8
#framebuffer_tag:
#	.short MULTIBOOT_HEADER_TAG_FRAMEBUFFER
#	.short 1
#	.long (framebuffer_tag_end - framebuffer_tag)
#	.long 0
#	.long 0
#	.long 0
#framebuffer_tag_end:

.align 8
	.short MULTIBOOT_HEADER_TAG_END
	.short 0
	.long 8
header_end:

.global switch_protected

switch_protected:
	cli
	lgdt gdt
	mov %eax, %cr0
	or $1, %al
	mov %cr0, %eax

	jmp $0x8, $complete_flush
complete_flush:
	mov $0x8, %ax
	mov %ds, %ax
	mov %es, %ax
	mov %fs, %ax
	mov %gs, %ax
	mov %ss, %ax

	ret

kernel_init:
	call switch_protected

	ret

.global kernel_halt

kernel_halt:
	cli
halt_loop:
	hlt
	jmp halt_loop

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

	push %ebx
	push %eax
	call kernel_main
	call _fini

	call kernel_halt

.section .data

.align 8

gdt_start:
gdt_null:
	.quad 0

gdt_code:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10011010
	.byte 0b11001111
	.byte 0

gdt_data:
	.word 0xffff
	.word 0
	.byte 0
	.byte 0b10010010
	.byte 0b11001111
	.byte 0

#gdt_bios:
#	.word 0x10
#	.word 0
#	.byte 0
#	.byte 0b10010000
#	.byte 0b11000000
#	.byte 0

#gdt_code:
#	.word 0
#	.word 0x10000
#	.byte 0
#	.byte 0b10011010
#	.byte 0b11001000
#	.byte 0

#gdt_data:
#	.word 0xffff
#	.word 0
#	.byte 0
#	.byte 0b10000010
#	.byte 0b11001111
#	.byte 0b10000000

gdt:
	.word gdt - gdt_start - 1
	.long gdt_start

.section .bss

.align 8

stack_bottom:
	.skip STACK_SIZE
stack_top:
