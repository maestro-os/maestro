.global write
.global fork
.global _exit
.global getpid
.global waitpid
.global mmap
.global munmap

write:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $0x0, %eax
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

fork:
	push %ebp
	mov %esp, %ebp

	mov $0x1, %eax
	int $0x80

	mov %ebp, %esp
	pop %ebp
	ret

_exit:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x2, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

getpid:
	push %ebp
	mov %esp, %ebp

	mov $0x3, %eax
	int $0x80

	mov %ebp, %esp
	pop %ebp
	ret

getppid:
	push %ebp
	mov %esp, %ebp

	mov $0x4, %eax
	int $0x80

	mov %ebp, %esp
	pop %ebp
	ret

waitpid:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $0x5, %eax
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

mmap:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx
	push %esi
	push %edi
	push %ebp

	mov $0x6, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	mov 20(%ebp), %esi
	mov 24(%ebp), %edi
	mov 28(%ebp), %ebp
	int $0x80

	pop %ebp
	pop %edi
	pop %esi
	pop %edx
	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

munmap:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $0x7, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
