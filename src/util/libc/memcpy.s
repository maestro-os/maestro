.global memcpy
.type memcpy, @function

memcpy:
	push %esi
	push %edi

	mov 12(%esp), %edi
	mov 16(%esp), %esi
	mov 20(%esp), %ecx

	mov %edi, %eax

	cmp $4, %ecx
	jc loop
	test $3, %edi
	jz loop

pad:
	movsb
	dec %ecx
	test $3, %edi
	jnz pad

loop:
	mov %ecx, %edx
	shr $2, %ecx
	rep movsl
	and $3, %edx
	jz end

remain:
	movsb
	dec %edx
	jnz remain

end:
	pop %edi
	pop %esi
	ret
