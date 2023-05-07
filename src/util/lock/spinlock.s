.section .text

.global spin_lock
.global spin_unlock

.type spin_lock, @function
.type spin_unlock, @function

/*
 * Locks the given spinlock. If the spinlock is already locked, the thread shall wait until it becomes available.
 */
spin_lock:
	mov 4(%esp), %ecx

spin:
	mov $1, %eax
	xchg %eax, (%ecx)
	test %eax, %eax
	pause
	jnz spin

	ret

/*
 * Unlocks the given spinlock. Does nothing if the spinlock is already unlocked.
 */
spin_unlock:
	mov 4(%esp), %ecx

	movl $0, (%ecx)

	ret
