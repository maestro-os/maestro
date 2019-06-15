.text

.global paging_enable
.global paging_disable

paging_enable:
	push %ebp
	mov %esp, %ebp
	mov 8(%esp), %eax
	mov %eax, %cr3
	mov %cr0, %eax
	or $0x80000000, %eax
	mov %eax, %cr0
	mov %ebp, %esp
	pop %ebp

	ret

tlb_reload:
	movl %cr3, %eax
	movl %eax, %cr3

	ret

paging_disable:
	mov %cr0, %eax
	or $(~0x80000000), %eax
	mov %eax, %cr0

	ret
