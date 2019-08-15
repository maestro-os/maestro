# Interrupts

Interrupts allows the kernel to be notified when an event requires its attention.

Each interrupt is associated with a vector index.

Indexes from `0x0` to `0x1f` included are fired to throw an exception and might result in sending a signal to a process or kernel panic (depending on the situation).

Indexes from `0x20` to `0x2f` are events coming from the outside of the CPU.
Here are the ones that are used:
- `0x20`: PIT interrupts (see **pit.md**)
- `0x21`: Keyboard input

Index `0x80` is for syscalls (see **syscalls.md**)

At the end of an interrupt, the kernel sends an EOI (End Of Interrupt) to the PIC in order for the CPU to be able to receive further interrupts.
