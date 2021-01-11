// TODO doc

.section .text

.global idt_load
.extern end_of_interrupt

/*
 * TODO doc
 */
.macro IRQ	n
.global irq\n

irq\n:
	sub $40, %esp
	call get_regs

	push %esp
	push $0
	push $\n
	call event_handler
	add $12, %esp

	call restore_regs
	add $40, %esp

	iret
.endm

.global irq0

IRQ 0
IRQ 1
IRQ 2
IRQ 3
IRQ 4
IRQ 5
IRQ 6
IRQ 7
IRQ 8
IRQ 9
IRQ 10
IRQ 11
IRQ 12
IRQ 13
IRQ 14
IRQ 15

idt_load:
	mov 4(%esp), %edx
	lidt (%edx)
	ret
