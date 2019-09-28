.global context_switch
.global kernel_switch

context_switch:
	cli
	mov %esp, %ebp
	mov $stack_top, %esp # TODO remove?

	mov 8(%ebp), %eax
	mov %ax, %ds
	mov %ax, %es
	mov %ax, %fs
	mov %ax, %gs

	mov 4(%ebp), %eax
	mov 20(%eax), %ebx
	mov 24(%eax), %ecx
	mov 28(%eax), %edx
	mov 32(%eax), %esi
	mov 36(%eax), %edi

	push 8(%ebp)
	push 4(%eax)
	pushf
	orl $512, (%esp)
	push 12(%ebp)
	push 8(%eax)

	push 16(%ebp)
	mov (%eax), %ebp
	mov 16(%eax), %eax

	pusha
	push 32(%esp)
	call paging_enable
	add $4, %esp

	push $0x0
	call pic_EOI
	add $4, %esp
	popa
	add $4, %esp

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

	pusha
	push $0x0
	call pic_EOI
	add $4, %esp
	popa

	sti
	ret
