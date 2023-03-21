.global interrupt_is_enabled
.global end_tick

/*
 * Tells whether interrupts are enabled or not.
 */
interrupt_is_enabled:
	pushf
	pop %eax
	and $0x200, %eax
	ret

/*
 * Ends the current tick on the current CPU.
 */
end_tick:
	int $0x20
	ret
