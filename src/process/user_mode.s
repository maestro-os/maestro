.global switch_user_mode

switch_user_mode:
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
	# TODO Push function ptr?
	iret
