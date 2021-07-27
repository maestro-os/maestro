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

	mov $15, %eax
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

	mov $8, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

_exit:
	push %ebp
	mov %esp, %ebp

	mov $16, %eax
	mov 8(%ebp), %ebx
	int $0x80

	ud2

fork:
	mov $17, %eax
	int $0x80
	ret

waitpid:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $19, %eax
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
	mov $24, %eax
	int $0x80
	ret

getppid:
	mov $25, %eax
	int $0x80
	ret

signal:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $32, %eax
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

	mov $33, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
