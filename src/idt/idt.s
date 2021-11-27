/*
 * This file contains macros and functions created by these macros to handle interruptions.
 */

.include "src/process/regs/regs.s"

.global idt_load
.extern end_of_interrupt

.section .text

/*
 * This macro creates a function to handle an error interrupt that does **not** pass an additional error code.
 * `n` is the id in the interrupt vector.
 */
.macro ERROR_NOCODE	n
.global error\n

error\n:
	push %ebp
	mov %esp, %ebp

	# Allocating space for registers and retrieving them
GET_REGS \n

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp # regs
	push %eax # ring
	push $0 # code
	push $\n # id
	call event_handler
	add $16, %esp

END_INTERRUPT
.endm



/*
 * This macro creates a function to handle an error interrupt that passes an additional error code.
 * `n` is the id in the interrupt vector.
 */
.macro ERROR_CODE	n
.global error\n

error\n:
	# Retrieving the error code and writing it after the stack pointer so that it can be retrieved
	# after the stack frame
	push %eax
	mov 4(%esp), %eax
	mov %eax, -4(%esp)
	pop %eax

	# Removing the code from its previous location on the stack
	add $4, %esp

	push %ebp
	mov %esp, %ebp

	# Allocating space for the error code
	push -8(%esp)

	# Allocating space for registers and retrieving them
GET_REGS \n

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp # regs
	push %eax # ring
	push (REGS_SIZE + 8)(%esp) # code
	push $\n # id
	call event_handler
	add $16, %esp

	# Freeing the space allocated for the error code
	add $4, %esp

END_INTERRUPT
.endm



/*
 * This macro creates a function to handle a regular interruption.
 * `n` is the id of the IRQ.
 */
.macro IRQ	n
.global irq\n

irq\n:
	push %ebp
	mov %esp, %ebp

	# Allocating space for registers and retrieving them
GET_REGS irq_\n

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp # regs
	push %eax # ring
	push $0 # code
	push $(\n + 0x20) # id
	call event_handler
	add $16, %esp

	push $(\n + 0x20)
	call end_of_interrupt
	add $4, %esp

END_INTERRUPT
.endm



/*
 * Creating the handlers for every errors.
 */
ERROR_NOCODE 0
ERROR_NOCODE 1
ERROR_NOCODE 2
ERROR_NOCODE 3
ERROR_NOCODE 4
ERROR_NOCODE 5
ERROR_NOCODE 6
ERROR_NOCODE 7
ERROR_CODE 8
ERROR_NOCODE 9
ERROR_CODE 10
ERROR_CODE 11
ERROR_CODE 12
ERROR_CODE 13
ERROR_CODE 14
ERROR_NOCODE 15
ERROR_NOCODE 16
ERROR_CODE 17
ERROR_NOCODE 18
ERROR_NOCODE 19
ERROR_NOCODE 20
ERROR_NOCODE 21
ERROR_NOCODE 22
ERROR_NOCODE 23
ERROR_NOCODE 24
ERROR_NOCODE 25
ERROR_NOCODE 26
ERROR_NOCODE 27
ERROR_NOCODE 28
ERROR_NOCODE 29
ERROR_CODE 30
ERROR_NOCODE 31

/*
 * Creating the handlers for every IRQs.
 */
IRQ 0
IRQ 1
IRQ 2
IRQ 3
IRQ 4
IRQ 5
IRQ 6
IRQ 7
IRQ 8
IRQ 9
IRQ 10
IRQ 11
IRQ 12
IRQ 13
IRQ 14
IRQ 15



/*
 * This function takes the IDT given as argument and loads it.
 */
idt_load:
	mov 4(%esp), %edx
	lidt (%edx)
	ret
