/*
 * This file contains macros and functions created by these macros to handle interruptions.
 */

.section .text

.global error_handler
.global idt_load
.extern end_of_interrupt

/*
 * This macro stores the values of every registers after an interruption was triggered.
 * It is required that the caller allocate some memory (the size of the registers storing structure) before calling.
 * The stack frame is used as a reference to place the register values.
 */
.macro GET_REGS n
	mov %edi, -0x4(%ebp)
	mov %esi, -0x8(%ebp)
	mov %edx, -0xc(%ebp)
	mov %ecx, -0x10(%ebp)
	mov %ebx, -0x14(%ebp)
	mov %eax, -0x18(%ebp)

	mov 12(%ebp), %eax
	mov %eax, -0x1c(%ebp) # eflags
	mov 4(%ebp), %eax
	mov %eax, -0x20(%ebp) # eip

	cmpl $0x8, 8(%ebp)
	je ring0_\n
	jmp ring3_\n

ring0_\n:
	mov %ebp, %eax
	add $16, %eax
	mov %eax, -0x24(%ebp) # esp
	jmp esp_end_\n

ring3_\n:
	mov 16(%ebp), %eax
	mov %eax, -0x24(%ebp) # esp

esp_end_\n:
	mov (%ebp), %eax
	mov %eax, -0x28(%ebp) # ebp
.endm



/*
 * This macro is meant to be called before the `iret` instruction.
 * It restores the values of the registers that are not updated by the `iret` instruction.
 * The values are taken from the structure that was previously allocated on the stack for the macro `GET_REGS`.
 * The function is not relinquishing the space taken by the structure on the stack.
 */
.macro RESTORE_REGS
	mov -0x4(%ebp), %edi
	mov -0x8(%ebp), %esi
	mov -0xc(%ebp), %edx
	mov -0x10(%ebp), %ecx
	mov -0x14(%ebp), %ebx
	mov -0x18(%ebp), %eax
.endm



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
	sub $40, %esp
GET_REGS \n

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp
	push %eax
	push $0
	push $\n
	call event_handler
	add $16, %esp

	# Restoring registers and freeing the allocated stack space
RESTORE_REGS
	add $40, %esp

	mov %ebp, %esp
	pop %ebp
	iret
.endm



/*
 * This macro creates a function to handle an error interrupt that passes an additional error code.
 * `n` is the id in the interrupt vector.
 */
.macro ERROR_CODE	n
.global error\n

error\n:
	# Copying the error code after the stack frame and popping it
	sub $4, %esp
	push %eax
	mov 8(%esp), %eax
	mov %eax, -36(%esp)
	pop %eax
	add $8, %esp

	push %ebp
	mov %esp, %ebp

	# Allocating space for registers and retrieving them
	sub $40, %esp
GET_REGS \n

	# Retrieving the error code on the stack
	sub $4, %esp

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp
	push %eax
	push 8(%esp)
	push $\n
	call event_handler
	add $16, %esp

	# Freeing the space allocated for the error code
	add $4, %esp

	# Restoring registers and freeing the allocated stack space
RESTORE_REGS
	add $40, %esp

	mov %ebp, %esp
	pop %ebp
	iret
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
	sub $40, %esp
GET_REGS irq_\n

	# Getting the ring
	mov 8(%ebp), %eax
	and $0b11, %eax

	# Pushing arguments to call event_handler
	push %esp
	push %eax
	push $0
	push $(\n + 0x20)
	call event_handler
	add $16, %esp

	push $(\n + 0x20)
	call end_of_interrupt
	add $4, %esp

	# Restoring registers and freeing the allocated stack space
RESTORE_REGS
	add $40, %esp

	mov %ebp, %esp
	pop %ebp
	iret
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
