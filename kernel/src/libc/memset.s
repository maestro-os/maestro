/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

.section .text

.global memset
.type memset, @function

# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT
memset:
	mov 12(%esp),%ecx
	cmp $62,%ecx
	ja 2f

	mov 8(%esp),%dl
	mov 4(%esp),%eax
	test %ecx,%ecx
	jz 1f

	mov %dl,%dh

	mov %dl,(%eax)
	mov %dl,-1(%eax,%ecx)
	cmp $2,%ecx
	jbe 1f

	mov %dx,1(%eax)
	mov %dx,(-1-2)(%eax,%ecx)
	cmp $6,%ecx
	jbe 1f

	shl $16,%edx
	mov 8(%esp),%dl
	mov 8(%esp),%dh

	mov %edx,(1+2)(%eax)
	mov %edx,(-1-2-4)(%eax,%ecx)
	cmp $14,%ecx
	jbe 1f

	mov %edx,(1+2+4)(%eax)
	mov %edx,(1+2+4+4)(%eax)
	mov %edx,(-1-2-4-8)(%eax,%ecx)
	mov %edx,(-1-2-4-4)(%eax,%ecx)
	cmp $30,%ecx
	jbe 1f

	mov %edx,(1+2+4+8)(%eax)
	mov %edx,(1+2+4+8+4)(%eax)
	mov %edx,(1+2+4+8+8)(%eax)
	mov %edx,(1+2+4+8+12)(%eax)
	mov %edx,(-1-2-4-8-16)(%eax,%ecx)
	mov %edx,(-1-2-4-8-12)(%eax,%ecx)
	mov %edx,(-1-2-4-8-8)(%eax,%ecx)
	mov %edx,(-1-2-4-8-4)(%eax,%ecx)

1:	ret 	

2:	movzbl 8(%esp),%eax
	mov %edi,12(%esp)
	imul $0x1010101,%eax
	mov 4(%esp),%edi
	test $15,%edi
	mov %eax,-4(%edi,%ecx)
	jnz 2f

1:	shr $2, %ecx
	rep
	stosl
	mov 4(%esp),%eax
	mov 12(%esp),%edi
	ret
	
2:	xor %edx,%edx
	sub %edi,%edx
	and $15,%edx
	mov %eax,(%edi)
	mov %eax,4(%edi)
	mov %eax,8(%edi)
	mov %eax,12(%edi)
	sub %edx,%ecx
	add %edx,%edi
	jmp 1b
