.global spin_lock
.global spin_unlock

/*
 * Locks the given spinlock.
 */
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

/*
 * Unlocks the given spinlock.
 */
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
