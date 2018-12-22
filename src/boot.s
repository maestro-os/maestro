.set MAGIC,			0xE85250D6
.set ARCHITECTURE,	0
.set HEADER_LENGTH,	16
.set CHECKSUM,		-(MAGIC + ARCHITECTURE + HEADER_LENGTH)
.set TAGS,			0

.section .multiboot

.align 4
.long MAGIC
.long ARCHITECTURE
.long HEADER_LENGTH
.long CHECKSUM
.long TAGS

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

kernel_halt:
	cli
halt_loop:
	hlt
	jmp halt_loop

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
	call kernel_halt

.size _start, . - _start
