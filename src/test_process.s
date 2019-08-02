.global write

write:
	mov $0x0, %eax
	mov 4(%esp), %ebx
	mov 8(%esp), %ecx
	mov 12(%esp), %edx
	int $0x80

	ret
