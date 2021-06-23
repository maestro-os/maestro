/*
 * This file implements functions related to the CPUID instruction (x86).
 */

.global msr_exist
.global msr_read
.global msr_write

.global get_current_apic

.section .text

/*
 * Tells whether MSR exist on the current core.
 */
msr_exist:
	push %ebx

	mov $1, %eax
	cpuid
	shr $5, %ebx
	and $1, %ebx

	mov %ebx, %eax
	pop %ebx
	ret

/*
 * Reads the value of the given MSR.
 */
msr_read:
	push %ebp
	mov %esp, %ebp

	mov 16(%ebp), %ecx # msr
	rdmsr
	mov 12(%ebp), %ecx # lo
	mov %eax, (%ecx)
	mov 8(%ebp), %ecx # hi
	mov %edx, (%ecx)

	mov %ebp, %esp
	pop %ebp
	ret

/*
 * Writes the given value to the given MSR.
 */
msr_write:
	push %ebp
	mov %esp, %ebp

	mov 16(%ebp), %ecx # msr
	mov 12(%ebp), %eax # lo
	mov 8(%ebp), %edx # hi
	wrmsr

	mov %ebp, %esp
	pop %ebp
	ret

/*
 * Returns the current CPU id.
 */
get_current_apic:
	push %ebx

	mov $1, %eax
	cpuid
	shr $24, %ebx

	mov %ebx, %eax
	pop %ebx
	ret