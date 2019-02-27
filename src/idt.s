global irq0
global irq1
global irq2
global irq3
global irq4
global irq5
global irq6
global irq7
global irq8
global irq9
global irq10
global irq11
global irq12
global irq13
global irq14
global irq15

global load_idt

global irq0_handler
global irq1_handler
global irq2_handler
global irq3_handler
global irq4_handler
global irq5_handler
global irq6_handler
global irq7_handler
global irq8_handler
global irq9_handler
global irq10_handler
global irq11_handler
global irq12_handler
global irq13_handler
global irq14_handler
global irq15_handler

irq0:
	pusha
	call irq0_handler
	popa

irq1:
	pusha
	call irq1_handler
	popa

irq2:
	pusha
	call irq2_handler
	popa

irq3:
	pusha
	call irq3_handler
	popa

irq4:
	pusha
	call irq4_handler
	popa

irq5:
	pusha
	call irq5_handler
	popa

irq6:
	pusha
	call irq6_handler
	popa

irq7:
	pusha
	call irq7_handler
	popa

irq8:
	pusha
	call irq8_handler
	popa

irq9:
	pusha
	call irq9_handler
	popa

irq10:
	pusha
	call irq10_handler
	popa

irq11:
	pusha
	call irq11_handler
	popa

irq12:
	pusha
	call irq12_handler
	popa

irq13:
	pusha
	call irq13_handler
	popa

irq14:
	pusha
	call irq14_handler
	popa

irq15:
	pusha
	call irq15_handler
	popa

load_idt:
	# TODO
	sti
	ret
