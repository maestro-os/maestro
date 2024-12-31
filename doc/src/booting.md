# Booting

The kernel booting sequence is supervised by the **Multiboot2** standard.

### Command line arguments

Multiboot allows passing command line arguments to the kernel at boot. The following arguments are supported:

- `-root <major> <minor>` (required): Tells the major/minor version numbers of the VFS's root device
- `-init <path>`: Tells the path of the binary to be run as the first process instead of the default path
- `-silent`: Tells the kernel not to show logs on screen while booting

## Memory remapping

The kernel is divided into two parts:
- Booting stub, located at `0x100000` on virtual memory
- Main kernel code, located at different positions depending on the architecture in virtual memory:
  - x86: `0xc0200000`
  - x86_64: `0xffff800000200000`

Because GRUB loads the whole kernel at `0x100000`, it is required to remap the memory to use the main code of the kernel. This is done through paging.

TODO: Add schematics of memory mapping

The mapping of memory at `0x100000` is removed later because it is not required anymore.

## Init process

The init process is the first program to be run by the kernel, which is in charge of initializing the system.

The program must be located at `/sbin/init`, or another path if specified as a command line argument.

The init process has PID `1` and is running as the superuser (uid: `0`, gid: `0`). If this process is killed, the kernel panics.
