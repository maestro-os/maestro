/*
 * This file implements functions for the structure containing registers's states
 */

// The size in bytes of the structure storing the registers' states
.set REGS_SIZE, 554

/*
 * This macro stores the values of every registers after an interruption was triggered.
 * Memory is allocated on the stack to store the values.
 * The stack frame is used as a reference to place the register values.
 */
.macro GET_REGS n
	# Allocating space on the stack to store the registers
	sub $REGS_SIZE, %esp

	# Filling registers in the structure
	mov %fs, 0x2c(%esp)
	mov %gs, 0x28(%esp)
	mov %edi, 0x24(%esp)
	mov %esi, 0x20(%esp)
	mov %edx, 0x1c(%esp)
	mov %ecx, 0x18(%esp)
	mov %ebx, 0x14(%esp)
	mov %eax, 0x10(%esp)

	# Saving the fx state
	mov %esp, %eax
	add $0x2a, %eax
	push %eax
	call save_fxstate
	add $4, %esp

	mov 12(%ebp), %eax
	mov %eax, 0xc(%esp) # eflags
	mov 4(%ebp), %eax
	mov %eax, 0x8(%esp) # eip

	cmpl $0x8, 8(%ebp)
	je ring0_\n
	jmp ring3_\n

# If the interruption was raised while executing on ring 0
ring0_\n:
	mov %ebp, %eax
	add $16, %eax
	mov %eax, 0x4(%esp) # esp
	jmp esp_end_\n

# If the interruption was raised while executing on ring 3
ring3_\n:
	mov 16(%ebp), %eax
	mov %eax, 0x4(%esp) # esp

esp_end_\n:
	mov (%ebp), %eax
	mov %eax, 0x0(%esp) # ebp
.endm



/*
 * This macro restores the registers' states, frees the space allocated by the function GET_REGS,
 * then terminates the interrupt handler to restore the previous context.
 */
.macro END_INTERRUPT
	# Restoring the fx state
	mov %esp, %eax
	add $0x2a, %eax
	push %eax
	call restore_fxstate
	add $4, %esp

	# Restoring registers
	mov 0x2c(%esp), %fs
	mov 0x28(%esp), %gs
	mov 0x24(%esp), %edi
	mov 0x20(%esp), %esi
	mov 0x1c(%esp), %edx
	mov 0x18(%esp), %ecx
	mov 0x14(%esp), %ebx
	mov 0x10(%esp), %eax

	# Freeing the space allocated on the stack
	add $REGS_SIZE, %esp

	# Restoring the context
	mov %ebp, %esp
	pop %ebp
	iret
.endm
