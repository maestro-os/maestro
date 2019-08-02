.global write
.global fork
.global _exit
.global waitpid

write:
	mov $0x0, %eax
	mov 4(%esp), %ebx
	mov 8(%esp), %ecx
	mov 12(%esp), %edx
	int $0x80

	ret

fork:
	# TODO
	ret

_exit:
	# TODO
	ret

waitpid:
	# TODO
	ret
