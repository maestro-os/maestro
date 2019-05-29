.global switch_usermode

switch_usermode:
	mov $USER_DATA_OFFSET, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	mov %esp, %eax
	push $USER_DATA_OFFSET
	push %eax
	pushf
	push $USER_CODE_OFFSET
	push $usermode_jump
	iret

usermode_jump:
	ret
