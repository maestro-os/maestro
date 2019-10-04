.global interrupt_is_enabled

interrupt_is_enabled:
	pushf
	pop %eax
	or $0x200, %eax
	ret
