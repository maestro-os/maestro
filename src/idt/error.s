.global error0
.global error1
.global error2
.global error3
.global error4
.global error5
.global error6
.global error7
.global error8
.global error9
.global error10
.global error11
.global error12
.global error13
.global error14
.global error15
.global error16
.global error17
.global error18
.global error19
.global error20
.global error21
.global error22
.global error23
.global error24
.global error25
.global error26
.global error27
.global error28
.global error29
.global error30
.global error31

.global error_handler

error0:
	pusha
	mov $0x0, %eax
	call error_handler
	popa
	iret

error1:
	pusha
	mov $0x1, %eax
	call error_handler
	popa
	iret

error2:
	pusha
	mov $0x2, %eax
	call error_handler
	popa
	iret

error3:
	pusha
	mov $0x3, %eax
	call error_handler
	popa
	iret

error4:
	pusha
	mov $0x4, %eax
	call error_handler
	popa
	iret

error5:
	pusha
	mov $0x5, %eax
	call error_handler
	popa
	iret

error6:
	pusha
	mov $0x6, %eax
	call error_handler
	popa
	iret

error7:
	pusha
	mov $0x7, %eax
	call error_handler
	popa
	iret

error8:
	pusha
	mov $0x8, %eax
	call error_handler
	popa
	iret

error9:
	pusha
	mov $0x9, %eax
	call error_handler
	popa
	iret

error10:
	pusha
	mov $0xa, %eax
	call error_handler
	popa
	iret

error11:
	pusha
	mov $0xb, %eax
	call error_handler
	popa
	iret

error12:
	pusha
	mov $0xc, %eax
	call error_handler
	popa
	iret

error13:
	pusha
	mov $0xd, %eax
	call error_handler
	popa
	iret

error14:
	pusha
	mov $0xe, %eax
	call error_handler
	popa
	iret

error15:
	pusha
	mov $0xf, %eax
	call error_handler
	popa
	iret

error16:
	pusha
	mov $0x10, %eax
	call error_handler
	popa
	iret

error17:
	pusha
	mov $0x11, %eax
	call error_handler
	popa
	iret

error18:
	pusha
	mov $0x12, %eax
	call error_handler
	popa
	iret

error19:
	pusha
	mov $0x13, %eax
	call error_handler
	popa
	iret

error20:
	pusha
	mov $0x14, %eax
	call error_handler
	popa
	iret

error21:
	pusha
	mov $0x15, %eax
	call error_handler
	popa
	iret

error22:
	pusha
	mov $0x16, %eax
	call error_handler
	popa
	iret

error23:
	pusha
	mov $0x17, %eax
	call error_handler
	popa
	iret

error24:
	pusha
	mov $0x18, %eax
	call error_handler
	popa
	iret

error25:
	pusha
	mov $0x19, %eax
	call error_handler
	popa
	iret

error26:
	pusha
	mov $0x1a, %eax
	call error_handler
	popa
	iret

error27:
	pusha
	mov $0x1b, %eax
	call error_handler
	popa
	iret

error28:
	pusha
	mov $0x1c, %eax
	call error_handler
	popa
	iret

error29:
	pusha
	mov $0x1d, %eax
	call error_handler
	popa
	iret

error30:
	pusha
	mov $0x1e, %eax
	call error_handler
	popa
	iret

error31:
	pusha
	mov $0x1f, %eax
	call error_handler
	popa
	iret
