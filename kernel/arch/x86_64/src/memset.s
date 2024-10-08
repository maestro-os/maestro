# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

.global memset
.type memset, @function

memset:
    movzbq rax, sil
    mov r8, 0x101010101010101
    imul rax, r8

    cmp rdx, 126
    ja 2f

    test edx, edx
    jz 1f

    mov [rdi], sil
    mov [rdi + rdx -1], [sil]
    cmp edx, 2
    jbe 1f

    mov [rdi + 1], ax
    mov [rdi + rdx -1-2], ax
    cmp edx, 6
    jbe 1f

    mov [rdi + 1+2], eax
    mov [rdi + rdx -1-2-4], eax
    cmp edx, 14
    jbe 1f

    mov [rdi + 1+2+4], rax
    mov [rdi + rdx -1-2-4-8], rax
    cmp edx, 30
    jbe 1f

    mov [rdi + 1+2+4+8], rax
    mov [rdi + 1+2+4+8+8], rax
    mov [rdi + rdx -1-2-4-8-16], rax
    mov [rdi + rdx -1-2-4-8-8], rax
    cmp edx, 62
    jbe 1f

    mov [rdi + 1+2+4+8+16], rax
    mov [rdi + 1+2+4+8+16+8], rax
    mov [rdi + 1+2+4+8+16+16], rax
    mov [rdi + 1+2+4+8+16+24], rax
    mov [rdi + rdx -1-2-4-8-16-32], rax
    mov [rdi + rdx -1-2-4-8-16-24], rax
    mov [rdi + rdx -1-2-4-8-16-16], rax
    mov [rdi + rdx -1-2-4-8-16-8], rax

1:
    mov rax, rdi
    ret

2:
    test edi, 15
    mov r8, rdi
    mov [rdi + rdx - 8], rax
    mov rcx, rdx
    jnz 2f

1:
    shr rcx, 3
    rep
    stosq
    mov rax, r8
    ret

2:
    xor edx,edx
    sub edx, edi
    and edx, 15
    mov [rdi], rax
    mov [rdi + 8], rax
    sub rcx, rdx
    add rdi, rdx
    jmp 1b
