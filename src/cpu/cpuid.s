/*
 * File implementing CPUID-related features.
 */

.global cpuid_has_sse
.global get_hwcap

.type cpuid_has_sse, @function
.type get_hwcap, @function

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

get_hwcap:
	push %ebx

	mov $0x1, %eax
	cpuid
	mov %edx, %eax

	pop %ebx
	ret
