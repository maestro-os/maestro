.global spin_lock
.global spin_unlock

spin_lock:
	push %ebp
	mov %esp, %ebp

	push %eax
	push %ebx

spin:
	mov $1, %eax
	mov 8(%ebp), %ebx
	xchg %eax, (%ebx)
	test %eax, %eax
	jnz spin

	pop %ebx
	pop %eax

	mov %ebp, %esp
	pop %ebp
	ret

spin_unlock:
	push %ebp
	mov %esp, %ebp

	push %eax
	push %ebx

	xor %eax, %eax
	mov 8(%ebp), %ebx
	mov %eax, (%ebx)

	pop %ebx
	pop %eax

	mov %ebp, %esp
	pop %ebp
	ret
