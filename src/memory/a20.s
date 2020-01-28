.global check_a20

/*
 * Checks whether the a20 line is enabled or not.
 */
check_a20:
	pusha
	mov $0x888888, %edi
	mov $0x088888, %esi
	mov %edi, (%edi)
	mov %esi, (%esi)
	cmpsd
	popa
	jne a20_enabled
	xor %eax, %eax
	ret
a20_enabled:
	mov $1, %eax
	ret
