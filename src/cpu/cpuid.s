.set MANUFACTURER_ID_LENGTH,	12

.text

.global cpuid_init

cpuid_init:
	push %ebp
	mov %esp, %ebp

	xor %eax, %eax
	cpuid

	push %ebx
	mov 8(%ebp), %ebx
	mov %eax, (%ebx)

	mov 16(%ebp), %eax
	pop %ebx
	mov %ebx, (%eax)
	mov %ecx, 8(%eax)
	mov %edx, 16(%eax)

	mov %ebp, %esp
	pop %ebp
	ret
