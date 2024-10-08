# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

.intel_syntax noprefix

.section .text

.global memcpy
.global __memcpy_fwd
.hidden __memcpy_fwd
.type memcpy, @function

memcpy:
__memcpy_fwd:
	push esi
	push edi
	mov edi, [esp + 12]
	mov esi, [esp + 16]
	mov ecx, [esp + 20]
	mov eax, edi
	cmp ecx, 4
	jc 1f
	test edi, 3
	jz 1f
2:
	movsb
	dec ecx
	test edi, 3
	jnz 2b
1:
	mov edx, ecx
	shr ecx, 2
	rep movsd
	and edx, 3
	jz 1f
2:
	movsb
	dec edx
	jnz 2b
1:
	pop edi
	pop esi
	ret
