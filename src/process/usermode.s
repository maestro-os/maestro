.global switch_usermode

switch_usermode:
	mov gdt_user_data, %ax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	mov %esp, %eax
	push gdt_user_data
	push %eax
	pushf
	push gdt_user_code
	push usermode_jump
	iret

usermode_jump:
	ret
