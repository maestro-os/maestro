.global spin_lock
.global spin_unlock

spin_lock:
	push %ebp
	mov %esp, %ebp

spin:
	mov $1, %eax
	xchg %eax, 8(%esp)
	test %eax, %eax
	jnz spin

	mov %ebp, %esp
	pop %ebp
	ret

spin_unlock:
	push %ebp
	mov %esp, %ebp

	xor %eax, %eax
	xchg %eax, 8(%esp)

	mov %ebp, %esp
	pop %ebp
	ret
