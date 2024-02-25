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

/*
 * This file implements paging-related features.
 */

.section .text

.global paging_enable
.global paging_disable
.global invlpg
.global tlb_reload

.type paging_enable, @function
.type paging_disable, @function
.type invlpg, @function
.type tlb_reload, @function

/*
 * (x86) Enables paging using the specified page directory.
 */
paging_enable:
	push %ebp
	mov %esp, %ebp
	push %eax

	mov 8(%ebp), %eax
	mov %eax, %cr3
	mov %cr0, %eax
	or $0x80010000, %eax
	mov %eax, %cr0

	pop %eax
	mov %ebp, %esp
	pop %ebp
	ret

/*
 * (x86) Disables paging.
 */
paging_disable:
	push %eax
	mov %cr0, %eax
	and $(~0x80000000), %eax
	mov %eax, %cr0
	pop %eax
	ret

/*
 * (x86) Executes the invlpg for the given page address.
 */
invlpg:
	push %eax

	mov 8(%esp), %eax
	invlpg (%eax)

	pop %eax
	ret

/*
 * (x86) Reloads the Translate Lookaside Buffer.
 */
tlb_reload:
	push %eax

	movl %cr3, %eax
	movl %eax, %cr3

	pop %eax
	ret
