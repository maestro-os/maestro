.text

.global paging_enable
.global tlb_reload
.global cr2_get
.global cr3_get
.global paging_disable

paging_enable:
	push %ebp
	mov %esp, %ebp
	push %eax

	mov 8(%ebp), %eax
	mov %eax, %cr3
	mov %cr0, %eax
	or $0x80000000, %eax
	mov %eax, %cr0

	pop %eax
	mov %ebp, %esp
	pop %ebp
	ret

tlb_reload:
	push %eax
	movl %cr3, %eax
	movl %eax, %cr3
	pop %eax
	ret

cr2_get:
	mov %cr2, %eax
	ret

cr3_get:
	mov %cr3, %eax
	ret

paging_disable:
	push %eax
	mov %cr0, %eax
	and $(~0x80000000), %eax
	mov %eax, %cr0
	pop %eax
	ret
