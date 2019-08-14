.text

.global syscall

syscall:
	push %ebp
	push %edi
	push %esi
	push %edx
	push %ecx
	push %ebx
	push %eax

	mov $GDT_KERNEL_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push %esp
	call syscall_handler
	add $32, %esp
	push %eax

	xor %eax, %eax
	mov $GDT_USER_DATA_OFFSET, %ax
	or $3, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push $0x80
	call pic_EOI
	add $4, %esp

	pop %eax

	sti
	iret
