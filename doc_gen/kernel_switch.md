```
extern void kernel_switch(const regs_t *regs);
```

Switches context, staying in kernel mode.

When switching context, registers will be changed for the values in **regs**.

To switch to user mode, see **context_switch**.
