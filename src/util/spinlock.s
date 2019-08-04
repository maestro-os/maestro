.global spin_lock
.global spin_unlock

spin_lock:
	push %ebp
	mov %esp, %ebp

spin:
	mov $1, %eax
	mov 8(%ebp), %ebx
	xchg %eax, (%ebx)
	test %eax, %eax
	jnz spin

	mov %ebp, %esp
	pop %ebp
	ret

spin_unlock:
	push %ebp
	mov %esp, %ebp

	xor %eax, %eax
	mov 8(%ebp), %ebx
	mov %eax, (%ebx)

	mov %ebp, %esp
	pop %ebp
	ret
