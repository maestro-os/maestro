/*
 * File implementing CPUID-related features.
 */

.global cpuid_has_sse

.section .text

cpuid_has_sse:
	push %ebx

	mov $0x1, %eax
	cpuid
	shr $25, %edx
	and $0x1, %edx
	mov %edx, %eax

	pop %ebx
	ret
