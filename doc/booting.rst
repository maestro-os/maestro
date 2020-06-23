Booting
*******

The kernel booting sequence is supervised by the Multiboot2 standard. The recommended bootloader for it is GRUB2.



Multiboot2
==========

Multiboot is a very simple booting standard created by GNU. The specification is available here: `https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html`
Advantages of this standard on the kernelside include:

- Compatibility with GRUB, one of the most used bootloader
- Easy to implement
- Provides usefull infomations on the system, like memory mapping



Kernel boot sequence
====================

On x86, after execution has been passed to the kernel, the CPU is still running in real mode.
First, the kernel will setup a stack by changing the value into the ``%esp`` register.

Then the kernel switches to protected mode by loading a GDT (Global Descriptor Table).
Because segmentation is obsolete, then kernel defines segments that cover the whole memory.

Here is the list of the segments:

- Kernel code
- Kernel data
- User code
- User data
- TSS

All segments (except **TSS**) allow to read/write/execute in kernel and user mode on the whole memory space.

The **TSS** (Task Switch Segment) is an almost-obsolete segment that is required for task switching, its purpose is explained in section **Task Switching**.

After that, the kernel passes control to the main function.
