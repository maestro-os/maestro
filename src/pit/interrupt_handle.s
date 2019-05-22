.section .bss

.align 8

lock:
	.byte 0

.section .text

.global interrupt_handle
.global interrupt_done

interrupt_handle:
	mov $1, %al
	xchg %al, $lock

	ret

interrupt_done:
	xor %al, %al
	mov %al, $lock

	ret
