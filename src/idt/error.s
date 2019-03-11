.text

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
	push $0x0
	call error_handler
	popa
	iret

error1:
	pusha
	push $0x1
	call error_handler
	popa
	iret

error2:
	pusha
	push $0x2
	call error_handler
	popa
	iret

error3:
	pusha
	push $0x3
	call error_handler
	popa
	iret

error4:
	pusha
	push $0x4
	call error_handler
	popa
	iret

error5:
	pusha
	push $0x5
	call error_handler
	popa
	iret

error6:
	pusha
	push $0x6
	call error_handler
	popa
	iret

error7:
	pusha
	push $0x7
	call error_handler
	popa
	iret

error8:
	pusha
	push $0x8
	call error_handler
	popa
	iret

error9:
	pusha
	push $0x9
	call error_handler
	popa
	iret

error10:
	pusha
	push $0xa
	call error_handler
	popa
	iret

error11:
	pusha
	push $0xb
	call error_handler
	popa
	iret

error12:
	pusha
	push $0xc
	call error_handler
	popa
	iret

error13:
	pusha
	push $0xd
	call error_handler
	popa
	iret

error14:
	pusha
	push $0xe
	call error_handler
	popa
	iret

error15:
	pusha
	push $0xf
	call error_handler
	popa
	iret

error16:
	pusha
	push $0x10
	call error_handler
	popa
	iret

error17:
	pusha
	push $0x11
	call error_handler
	popa
	iret

error18:
	pusha
	push $0x12
	call error_handler
	popa
	iret

error19:
	pusha
	push $0x13
	call error_handler
	popa
	iret

error20:
	pusha
	push $0x14
	call error_handler
	popa
	iret

error21:
	pusha
	push $0x15
	call error_handler
	popa
	iret

error22:
	pusha
	push $0x16
	call error_handler
	popa
	iret

error23:
	pusha
	push $0x17
	call error_handler
	popa
	iret

error24:
	pusha
	push $0x18
	call error_handler
	popa
	iret

error25:
	pusha
	push $0x19
	call error_handler
	popa
	iret

error26:
	pusha
	push $0x1a
	call error_handler
	popa
	iret

error27:
	pusha
	push $0x1b
	call error_handler
	popa
	iret

error28:
	pusha
	push $0x1c
	call error_handler
	popa
	iret

error29:
	pusha
	push $0x1d
	call error_handler
	popa
	iret

error30:
	pusha
	push $0x1e
	call error_handler
	popa
	iret

error31:
	pusha
	push $0x1f
	call error_handler
	popa
	iret
