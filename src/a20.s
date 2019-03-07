.global check_a20

check_a20:
	pusha
	mov $0x112345, %edi
	mov $0x012345, %esi
	mov (%edi), %edi
	mov (%esi), %esi
	cmpsd
	popa
	jne a20_enabled
	xor %eax, %eax
	ret
a20_enabled:
	mov $1, %eax
	ret
