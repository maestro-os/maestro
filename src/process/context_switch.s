.global context_switch

context_switch:
	push %ebp
	mov %esp, %ebp

	mov 8(%esp), %ebx
	mov 16(%esp), %ecx

	mov $USER_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push $USER_DATA_OFFSET
	push %ebx
	pushf
	push $USER_CODE_OFFSET
	push %ecx

	iret

	# TODO Useful? \/

	mov %ebp, %esp
	pop %ebp

	ret
