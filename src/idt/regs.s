/*
 * This file contains functions to get the values of every registers when an interruption happens and to restore them when the interruption returns.
 */

.section .text

.global get_regs
.global restore_regs

/*
 * This function stores the values of every registers after an interruption was triggered.
 * It is required that the caller allocate some memory (the size of the registers storing structure) before calling.
 */
get_regs:
	mov %edi, -0x4(%ebp)
	mov %esi, -0x8(%ebp)
	mov %edx, -0xc(%ebp)
	mov %ecx, -0x10(%ebp)
	mov %ebx, -0x14(%ebp)
	mov %eax, -0x18(%ebp)

	mov 12(%ebp), %eax
	mov %eax, -0x1c(%ebp)
	mov 4(%ebp), %eax
	mov %eax, -0x20(%ebp)

	cmp $0x8, 8(%ebp)
	je ring0
	jmp ring3

ring0:
	mov %ebp, %eax
	add $16, %eax
	mov %eax, -0x24(%ebp)
	jmp esp_end

ring3:
	mov 16(%ebp), %eax
	mov %eax, -0x24(%ebp)

esp_end:
	mov (%ebp), %eax
	mov %eax, -0x28(%ebp)

	ret



/*
 * This function is meant to be called before the `iret` instruction.
 * It restores the values of the registers that are not updated from the `iret` instruction.
 * The values are taken from the structure that was previously allocated on the stack for the function `get_regs`.
 * The function is not relinquishing the space taken by the structure on the stack.
 */
restore_regs:
	mov -0x4(%ebp), %edi
	mov -0x8(%ebp), %esi
	mov -0xc(%ebp), %edx
	mov -0x10(%ebp), %ecx
	mov -0x14(%ebp), %ebx
	mov -0x18(%ebp), %eax
	ret
