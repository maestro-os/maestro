Booting
*******

The kernel booting sequence is supervised by the Multiboot2 standard. The recommended bootloader for it is GRUB2.



Multiboot2
==========

Multiboot is a very simple booting standard created by GNU. The specification is available `here <https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html>`_.
Advantages of this standard on the kernelside include:

- Compatibility with GRUB, one of the most used bootloader
- Easy to implement
- Provides usefull infomations on the system, like memory mapping



Command line arguments
----------------------

Multiboot allows passing command line arguments to the kernel at boot. The following arguments are supported:

- `-root <major> <minor>` (required): Tells the major/minor version numbers of the VFS's root device
- `-init <path>`: Tells the path of the binary to be run as the first process instead of the default path
- `-silent`: Tells the kernel not to show logs on screen while booting



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



Memory remapping
----------------

The kernel is divided into two parts:
- Booting stub: located at ``0x100000`` on virtual memory
- Main kernel code: located at ``0xc0200000`` on virtual memory

Because GRUB loads the whole kernel at ``0x100000``, it is required to remap the memory to use the main code of the kernel. This is done through paging.

TODO: Add schematics of memory mapping

The mapping of memory at ``0x100000`` is removed later because it is not required anymore.



Initialization
==============

Once the memory remapped, the kernel begins the initialization sequence:
- TTY initialization
- Interrupts initialization
- PIT initialization
- Multiboot informations reading
- Physical memory mapping reading
- Memory allocators initialization
- Kernel virtual memory initialization
- TODO
- Processes initialization
- Creating the first process
- Running the scheduler
