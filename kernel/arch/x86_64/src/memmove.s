# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

.intel_syntax noprefix

.section .text

.global memmove
.type memmove, @function

memmove:
    mov rax, rdi
    sub rax, rsi
    cmp rax, rdx
.hidden __memcpy_fwd
    jae __memcpy_fwd
    mov rcx, rdx
    lea rdi, [rdi + rdx - 1]
    lea rsi, [rdi + rdx - 1]
    std
    rep movsb
    cld
    lea rax, [rdi + 1]
    ret
