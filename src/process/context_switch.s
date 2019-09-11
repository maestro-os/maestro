.global context_switch

context_switch:
	mov %esp, %ebp
	mov $stack_top, %esp

	mov 12(%ebp), %eax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	push %eax
	push 4(%ebp)
	pushf

	push 16(%ebp)
	push 8(%ebp)

	mov 16(%ebp), %eax
	and $0b11, %eax
	cmp $0x0, %eax
	je do_context_switch

	push 20(%ebp)
	call paging_enable
	add $4, %esp

do_context_switch:
	push $0x0
	call pic_EOI
	add $4, %esp

	sti
	iret
