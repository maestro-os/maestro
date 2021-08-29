.text

.global syscall

syscall:
	cli
	push %ebp
	mov %esp, %ebp

	push %edi
	push %esi
	push %edx
	push %ecx
	push %ebx
	push %eax

	push 12(%ebp)
	push 4(%ebp)
	push 16(%ebp)
	push (%ebp)

	mov $GDT_KERNEL_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push %esp
	sti
	call syscall_handler
	cli
	add $4, %esp

	xor %ebx, %ebx
	mov $GDT_USER_DATA_OFFSET, %bx
	or $3, %bx
	mov %bx, %ds
	mov %bx, %es
	mov %bx, %fs
	mov %bx, %gs

	add $16, %esp

	pop %eax
	pop %ebx
	pop %ecx
	pop %edx
	pop %esi
	pop %edi

	mov %ebp, %esp
	pop %ebp
	sti
	iret
