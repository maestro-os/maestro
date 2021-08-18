.global open
.global read
.global write
.global close
.global _exit
.global fork
.global waitpid
.global getpid
.global getppid
.global signal
.global kill
.global socketpair

.global init_module
.global finit_module
.global delete_module

open:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $0, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

read:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx

	mov $14, %eax
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

socketpair:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx
	push %edx
	push %esi

	mov $34, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	mov 20(%ebp), %esi
	int $0x80

	pop %esi
	pop %edx
	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

init_module:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $37, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

finit_module:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $38, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

delete_module:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %ecx

	mov $39, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ecx
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
