.global write
.global close
.global _exit
.global fork
.global waitpid
.global getpid
.global getppid

write:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $8, %eax
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

close:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $4, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

_exit:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $9, %eax
	mov 8(%ebp), %ebx
	int $0x80
	ud2

fork:
	mov $10, %eax
	int $0x80
	ret

waitpid:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $11, %eax
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

getpid:
	mov $16, %eax
	int $0x80
	ret

getppid:
	mov $17, %eax
	int $0x80
	ret
