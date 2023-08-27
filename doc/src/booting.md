# Booting

## Multiboot2

The kernel booting sequence is supervised by the Multiboot2 standard. The recommended bootloader for it is GRUB2.

Multiboot is a booting standard created by GNU. The specification is available [here](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html).

Advantages of this standard on the kernelside include:
- Compatibility with GRUB, one of the most popular bootloader
- Easy to implement



### Command line arguments

Multiboot allows passing command line arguments to the kernel at boot. The following arguments are supported:

- `-root <major> <minor>` (required): Tells the major/minor version numbers of the VFS's root device
- `-init <path>`: Tells the path of the binary to be run as the first process instead of the default path
- `-silent`: Tells the kernel not to show logs on screen while booting



## Memory remapping

The kernel is divided into two parts:
- Booting stub: located at `0x100000` on virtual memory
- Main kernel code: located at `0xc0200000` on virtual memory

Because GRUB loads the whole kernel at `0x100000`, it is required to remap the memory to use the main code of the kernel. This is done through paging.

TODO: Add schematics of memory mapping

The mapping of memory at `0x100000` is removed later because it is not required anymore.



## Init process

The init process is the first program to be run by the kernel, which is in charge of initializing the system.

The program must be located at `/sbin/init`, or an other path if specified as a command line argument.

The init process has PID `1` and is running as the superuser (uid: `0`, gid: `0`). If this process is killed, the kernel panics.
