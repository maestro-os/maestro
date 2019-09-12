.global context_switch
.global kernel_switch

context_switch:
	cli
	mov %esp, %ebp
	mov $stack_top, %esp # TODO remove?

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

	push 20(%ebp)
	call paging_enable
	add $4, %esp

	push $0x0
	call pic_EOI
	add $4, %esp

	sti
	iret

kernel_switch:
	cli

	mov 4(%esp), %eax
	mov (%eax), %ebp
	mov 4(%eax), %esp
	push 8(%eax)
	push 12(%eax)
	popf
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi
	mov 16(%eax), %eax

	push $0x0
	call pic_EOI
	add $4, %esp

	sti
	ret
