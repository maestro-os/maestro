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

.text
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

#.align 8
#address_tag:
#	.short MULTIBOOT_HEADER_TAG_ADDRESS
#	.short 1
#	.long (address_tag_end - address_tag)
#	.long header
#	.long _start
#	.long 0
#	.long bss_end
#address_tag_end:

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

kernel_init:
	# TODO

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

.size _start, . - _start

.section .bss

.align 8

stack_bottom:
	.skip STACK_SIZE
stack_top:
bss_end:
