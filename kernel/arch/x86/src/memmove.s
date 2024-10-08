# Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

.intel_syntax noprefix

.section .text

.global memmove
.type memmove, @function

memmove:
    mov eax, [esp + 4]
    sub eax, [esp + 8]
    cmp eax, [esp + 12]
.hidden __memcpy_fwd
    jae __memcpy_fwd
    push esi
    push edi
    mov edi, [esp + 12]
    mov esi, [esp + 16]
    mov ecx, [esp + 20]
    lea edi, [edi + ecx - 1]
    lea esi, [esi + ecx - 1]
    std
    rep movsb
    cld
    lea eax, [edi + 1]
    pop edi
    pop esi
    ret
