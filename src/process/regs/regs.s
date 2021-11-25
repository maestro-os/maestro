/*
 * This file implements functions for the structure containing registers's states
 */

.global REGS_SIZE
.global GET_REGS
.global END_INTERRUPT

// The size in bytes of the structure storing the registers' states
.set REGS_SIZE, 48

/*
 * This macro stores the values of every registers after an interruption was triggered.
 * Memory is allocated on the stack to store the values.
 * The stack frame is used as a reference to place the register values.
 */
.macro GET_REGS n
	# Allocating space on the stack to store the registers
	sub $REGS_SIZE, %esp

	# Filling general-purpose registers in the structure
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

# If the interruption was raised while executing on ring 0
ring0_\n:
	mov %ebp, %eax
	add $16, %eax
	mov %eax, -0x24(%ebp) # esp
	jmp esp_end_\n

# If the interruption was raised while executing on ring 3
ring3_\n:
	mov 16(%ebp), %eax
	mov %eax, -0x24(%ebp) # esp

esp_end_\n:
	mov (%ebp), %eax
	mov %eax, -0x28(%ebp) # ebp
.endm



/*
 * This macro restores the registers' states, frees the space allocated by the function GET_REGS,
 * then terminates the interrupt handler to restore the previous context.
 */
.macro END_INTERRUPT
	# Restoring general-purpose registers
	mov -0x4(%ebp), %edi
	mov -0x8(%ebp), %esi
	mov -0xc(%ebp), %edx
	mov -0x10(%ebp), %ecx
	mov -0x14(%ebp), %ebx
	mov -0x18(%ebp), %eax

	# Freeing the space allocated on the stack
	add $REGS_SIZE, %esp

	# Restoring the context
	mov %ebp, %esp
	pop %ebp
	iret
.endm
