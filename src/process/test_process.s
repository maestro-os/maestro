.global open
.global read
.global write
.global close
.global _exit
.global fork
.global waitpid
.global getpid
.global getppid
.global mmap
.global munmap
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

	mov $0x0, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

read:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0xd, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

write:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0xe, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

close:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x7, %eax
	mov 8(%ebp), %ebx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

_exit:
	push %ebp
	mov %esp, %ebp

	mov $0x0f, %eax
	mov 8(%ebp), %ebx
	int $0x80

	ud2

fork:
	mov $0x10, %eax
	int $0x80
	ret

waitpid:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x12, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

getpid:
	mov $0x17, %eax
	int $0x80
	ret

getppid:
	mov $0x18, %eax
	int $0x80
	ret

mmap:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %esi
	push %edi
	push %ebp

	mov $0x1d, %eax
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
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

munmap:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x1e, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

signal:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x20, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

kill:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x21, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

socketpair:
	push %ebp
	mov %esp, %ebp

	push %ebx
	push %esi

	mov $0x22, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	mov 16(%ebp), %edx
	mov 20(%ebp), %esi
	int $0x80

	pop %esi
	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

init_module:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x25, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

finit_module:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x26, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret

delete_module:
	push %ebp
	mov %esp, %ebp

	push %ebx

	mov $0x27, %eax
	mov 8(%ebp), %ebx
	mov 12(%ebp), %ecx
	int $0x80

	pop %ebx

	mov %ebp, %esp
	pop %ebp
	ret
