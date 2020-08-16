.global kernel_wait
.global kernel_loop
.global kernel_halt

.global kernel_end

.section .text

/*
 * Makes the kernel wait for an interrupt.
 */
kernel_wait:
	sti
	hlt
	ret

/*
 * Enters the kernel loop, process every interrupt indefinitely.
 */
kernel_loop:
	sti
	hlt
	jmp kernel_loop

/*
 * Halts the kernel forever.
 */
kernel_halt:
	cli
	hlt
	jmp kernel_halt

.section .text

/*
 * The kernel begin symbol, giving the pointer to the begin of the kernel image
 * in the virtual memory. This memory location should never be accessed using
 * this symbol.
 */
kernel_begin:

.section .bss

/*
 * The kernel end symbol, giving the pointer to the end of the kernel image in
 * the virtual memory. This memory location should never be accessed using this
 * symbol.
 */
kernel_end:
