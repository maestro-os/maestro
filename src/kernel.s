.global kernel_begin

.global kernel_wait
.global kernel_loop
.global kernel_loop_reset
.global kernel_halt

.global kernel_end

.section .text

/*
 * The kernel begin symbol, giving the pointer to the begin of the kernel image
 * in the virtual memory. This memory location should never be accessed using
 * this symbol.
 */
kernel_begin:

/*
 * Makes the kernel wait for an interrupt, then returns.
 * This function enables interrupts.
 */
kernel_wait:
	sti
	hlt
	ret

/*
 * Enters the kernel loop and processes every interrupts indefinitely.
 */
kernel_loop:
	sti
	hlt
	jmp kernel_loop

/*
 * Resets the stack to the given value, then calls `kernel_loop`.
 */
kernel_loop_reset:
	mov 4(%esp), %esp
	jmp kernel_loop

/*
 * Halts the kernel until reboot.
 */
kernel_halt:
	cli
	hlt
	jmp kernel_halt

.section .bss

/*
 * The kernel end symbol, giving the pointer to the end of the kernel image in
 * the virtual memory. This memory location should never be accessed using this
 * symbol.
 */
kernel_end:
