.global context_switch

context_switch:
	mov $USER_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push $USER_DATA_OFFSET
	push 8(%esp)
	pushf
	push $USER_CODE_OFFSET
	push 16(%esp)

	iret
	ret
