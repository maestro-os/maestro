.global context_switch

context_switch:
	mov $GDT_USER_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push $GDT_USER_DATA_OFFSET
	push 4(%esp)
	pushf
	push $GDT_USER_CODE_OFFSET
	push 8(%esp)

	iret
