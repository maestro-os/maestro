.global context_switch

context_switch:
	push %ebp # TODO Remove junk on stack (possible leak on stack)
	mov %esp, %ebp

	xor %eax, %eax
	mov $GDT_USER_DATA_OFFSET, %ax
	or $3, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push %eax
	push 8(%ebp)

	pushf

	xor %eax, %eax
	mov $GDT_USER_CODE_OFFSET, %ax
	or $3, %ax
	push %eax
	push 12(%ebp)

	push $0x0
	call pic_EOI
	add $4, %esp

	sti
	iret
