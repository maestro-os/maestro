/*
 * This file implements the signal handler trampoline.
 * The trampoline is using the same stack as the normal process execution. However, the System V
 * ABI defines a region of the stack located after the allocated portion which is called the red
 * zone. This region must not be clobbered, thus the kernel adds an offset on the stack
 * corresponding to the size of the red zone.
 *
 * When the signal handler returns, the process returns directly to execution.
 */

.global signal_trampoline

.section .text

/*
 * The signal handler trampoline. The process resumes to this function when it received a signal.
 */
signal_trampoline:
	push %ebp
	mov %esp, %ebp

	# TODO Save every registers
	# TODO

	mov %ebp, %esp
	pop %ebp
	ret
