.section .text

.global irq0
.global irq1
.global irq2
.global irq3
.global irq4
.global irq5
.global irq6
.global irq7
.global irq8
.global irq9
.global irq10
.global irq11
.global irq12
.global irq13
.global irq14
.global irq15

.global idt_load

.extern pic_EOI
.extern irq1_handler
.extern irq2_handler
.extern irq3_handler
.extern irq4_handler
.extern irq5_handler
.extern irq6_handler
.extern irq7_handler
.extern irq8_handler
.extern irq9_handler
.extern irq10_handler
.extern irq11_handler
.extern irq12_handler
.extern irq13_handler
.extern irq14_handler
.extern irq15_handler

irq0:
	cli
	push %ebp
	mov %esp, %ebp
	pusha

	push %edi
	push %esi
	push %edx
	push %ecx
	push %ebx
	push %eax

	push 12(%ebp)
	push 4(%ebp)

	cmp $0x8, 8(%ebp)
	je ring0
	jmp ring3

ring0:
	mov %ebp, %eax
	add $32, %eax
	push %eax
	jmp esp_end

ring3:
	push 16(%ebp)

esp_end:
	push (%ebp)

	call ata_err_check

	push %esp
	call process_tick
	add $44, %esp

	push $0x0
	call pic_EOI
	add $4, %esp

	popa
	mov %ebp, %esp
	pop %ebp
	sti
	iret

irq1:
	pusha
	call irq1_handler
	popa
	sti
	iret

irq2:
	pusha
	call irq2_handler
	popa
	sti
	iret

irq3:
	pusha
	call irq3_handler
	popa
	sti
	iret

irq4:
	pusha
	call irq4_handler
	popa
	sti
	iret

irq5:
	pusha
	call irq5_handler
	popa
	sti
	iret

irq6:
	pusha
	call irq6_handler
	popa
	sti
	iret

irq7:
	pusha
	call irq7_handler
	popa
	sti
	iret

irq8:
	pusha
	call irq8_handler
	popa
	sti
	iret

irq9:
	pusha
	call irq9_handler
	popa
	sti
	iret

irq10:
	pusha
	call irq10_handler
	popa
	sti
	iret

irq11:
	pusha
	call irq11_handler
	popa
	sti
	iret

irq12:
	pusha
	call irq12_handler
	popa
	sti
	iret

irq13:
	pusha
	call irq13_handler
	popa
	sti
	iret

irq14:
	pusha
	call irq14_handler
	popa
	sti
	iret

irq15:
	pusha
	call irq15_handler
	popa
	sti
	iret

idt_load:
	mov 4(%esp), %edx
	lidt (%edx)
	ret
