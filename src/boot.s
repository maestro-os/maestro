.align 8

.set MULTIBOOT_MAGIC,			0xE85250D6
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

.global _start
.type _start, @function

header:
	.long MULTIBOOT_MAGIC
	.long MULTIBOOT_ARCHITECTURE
	.long HEADER_LENGTH
	.long CHECKSUM

address_tag:
	.short MULTIBOOT_HEADER_TAG_ADDRESS
	.short 1
	.long (address_tag_end - address_tag)
	.long header
	.long _start
	.long edata
	.long end
address_tag_end:

entry_address_tag:
	.short MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS
	.short 1
	.long (entry_address_tag_end - entry_address_tag)
	.long multiboot_entry
entry_address_tag_end:

framebuffer_tag:
	.short MULTIBOOT_HEADER_TAG_FRAMEBUFFER
	.short 1
	.long (framebuffer_tag_end - framebuffer_tag)
	.long 0
	.long 0
	.long 0
framebuffer_tag_end:

	.short MULTIBOOT_HEADER_TAG_END
	.short 0
	.long 8
header_end:

_start:
	jmp multiboot_entry

kernel_init:
	# TODO

	ret

multiboot_entry:
	mov $stack_top, %esp

	push %ebx
	call kernel_init
	call _init
	pop %ebx

	pushl $0
	popf

	push %ebx
	call kernel_main

	call _fini

	cli
halt_loop:
	hlt
	jmp halt_loop

	ret

.size _start, . - _start

.section .bss

.align 8

stack_bottom:
	.skip STACK_SIZE
stack_top:
edata:

end:
