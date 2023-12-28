.global kernel_begin
.global kernel_end

.global kernel_loop_reset
.type kernel_loop_reset, @function

.section .text

/*
 * The kernel begin symbol, giving the pointer to the begin of the kernel image
 * in the virtual memory.
 */
kernel_begin:

/*
 * Resets the stack to the given value, then halts until an interruption is triggered.
 */
kernel_loop_reset:
	mov 4(%esp), %esp
	mov $0, %ebp
loop:
    sti
    hlt
	jmp loop

.section .bss

/*
 * The kernel end symbol, giving the pointer to the end of the kernel image in
 * the virtual memory.
 */
kernel_end:
