```
extern void context_switch(const regs_t *regs,
	uint16_t data_selector, uint16_t code_selector);
```

Switches to another context.

When switching context, registers will be changed for the values in **regs**.
**data_selector** and **code_selector** must be valid segment selectors.

A segment selector is the offset of the desired GDT entry, OR-ed with the request privilege level (`0` = kernel, `3` = user).

**Warning**: This function cannot be used to switch contexts keeping the same privilege level, as the `iret` instruction won't change the stack if the privilege level stays the same.
To switch context while staying in kernel mode, see **kernel_switch**.
