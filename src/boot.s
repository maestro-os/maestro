.set MAGIC,			0xE85250D6
.set ARCHITECTURE,	0
.set HEADER_LENGTH,	(header_end - header)
.set CHECKSUM,		-(MAGIC + ARCHITECTURE + HEADER_LENGTH)

.section .multiboot

.align 8

header:
	.long MAGIC
	.long ARCHITECTURE
	.long HEADER_LENGTH
	.long CHECKSUM

info_req:
	.short 1
	.short 0
	.long (info_req_end - info_req)
	.long 8

info_req_end:
	.short 0
	.short 0
	.long 8

header_end:

.section .bss

.align 16
stack_bottom: .skip 16384
stack_top:

.section .text

.global _start
.type _start, @function

kernel_init:
	# TODO

	ret

_start:
	mov $stack_top, %esp

	push %ebx # TODO Usefull?
	call kernel_init
	call _init
	pop %ebx # TODO Usefull?

	mov %ebx, %esp
	call kernel_main

	call _fini

	cli
halt_loop:
	hlt
	jmp halt_loop

.size _start, . - _start
