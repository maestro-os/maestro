# TODO doc?

.global write
.global _exit
.global getpid
.global getppid

# TODO doc?
write:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $5, %eax
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
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $6, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

# TODO doc?
getpid:
	mov $7, %eax
	int $0x80
	ret

# TODO doc?
getppid:
	mov $8, %eax
	int $0x80
	ret
