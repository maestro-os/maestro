/*
 * Constants used by Multiboot2 to detect the kernel.
 */
.set MULTIBOOT_MAGIC,			0xe85250d6
.set MULTIBOOT_ARCHITECTURE,	0
.set HEADER_LENGTH,				(header_end - header)
.set CHECKSUM,					-(MULTIBOOT_MAGIC + MULTIBOOT_ARCHITECTURE + HEADER_LENGTH)

/*
 * Multiboot header tags constants.
 */
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

/*
 * The size of the kernel stack.
 */
.set STACK_SIZE,	32768

.global boot_stack_top

.extern switch_protected
.extern _init
.extern _fini

.section .boot.text

/*
 * The Multiboot2 kernel header.
 */
.align 8
header:
	.long MULTIBOOT_MAGIC
	.long MULTIBOOT_ARCHITECTURE
	.long HEADER_LENGTH
	.long CHECKSUM

/*
 * The entry tag, setting the entry point of the kernel.
 */
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

/*
 * The entry point for the kernel.
 */
multiboot_entry:
	mov $boot_stack_top, %esp
	xor %ebp, %ebp

	pushl $0
	popf

	push %eax
	push %ebx
	call a20_handle
	call switch_protected
	# TODO call _init
	call kernel_remap
	pop %ebx
	pop %eax

	mov $(0xc0000000 + boot_stack_top), %esp
	push %ebx
	push %eax
	call kernel_main
	add $12, %esp

	call kernel_halt
	# TODO call _fini



.section .boot.stack, "w"

.align 8

/*
 * The kernel stack.
 */
.skip STACK_SIZE
boot_stack_top:
