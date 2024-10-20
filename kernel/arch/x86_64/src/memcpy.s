# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

.intel_syntax noprefix

.section .text

.global memcpy
.global __memcpy_fwd
.hidden __memcpy_fwd
.type memcpy, @function

memcpy:
__memcpy_fwd:
    mov rax, rdi
    cmp rdx, 8
    jc 1f
    test edi, 7
    jz 1f
2:
    movsb
    dec rdx
    test edi, 7
    jnz 2b
1:
    mov rcx, rdx
    shr rcx, 3
    rep movsq
    and edx, 7
    jz 1f
2:
    movsb
    dec edx
    jnz 2b
1:
    ret
