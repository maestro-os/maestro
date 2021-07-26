.global write
.global close
.global _exit
.global fork
.global waitpid
.global getpid
.global getppid
.global signal
.global kill

write:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $13, %eax
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

	mov $6, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

_exit:
	push %ebp
	mov %esp, %ebp

	mov $14, %eax
	mov 8(%ebp), %ebx
	int $0x80

	ud2

fork:
	mov $15, %eax
	int $0x80
	ret

waitpid:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $17, %eax
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
	mov $22, %eax
	int $0x80
	ret

getppid:
	mov $23, %eax
	int $0x80
	ret

signal:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $30, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

kill:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $31, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
