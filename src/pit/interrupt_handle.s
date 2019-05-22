.section .bss

.align 8

lock:
	.word 0

.section .text

.global interrupt_handle
.global interrupt_done

interrupt_handle:
	mov $1, %eax
	xchg ($lock), %eax

	ret

interrupt_done:
	xor %eax, %eax
	mov %eax, ($lock)

	ret
