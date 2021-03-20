# TODO doc?

.global write
.global _exit

# TODO doc?
write:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $0, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	int $0x80

	pop %edx
	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

# TODO doc?
_exit:
	mov $1, %eax
	int $0x80
	ret
