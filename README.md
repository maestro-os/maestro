# Maestro

Maestro is a lightweight Unix-like kernel written in Rust.



## Compilation

The kernel currently supports the following architectures:

|--------------|------------------|-------------------|
| Architecture | C Cross-Compiler | Target triplet    |
|--------------|------------------|-------------------|
| x86          | i686-elf-gcc     | i686-unknown-none |
|--------------|------------------|-------------------|

The following dependencies are required for compilation:
- The C Cross-Compiler associated with the targeted platform
- Nightly Rust compiler
- (optional) grub-mkrescue, only to build an ISO file



### Environement variables

Some environement variables can be used to customize the compilation of the kernel. The list is the following:
- **KERNEL_ARCH** (default: `x86`): Specify the platform for which the kernel will be compiled. The list of available platform is located in the `arch/` directory.
- **KERNEL_MODE** (default: `debug`): The mode of the kernel. Either `debug` or `release`.
- **KERNEL_TEST** (default: `false`): Tells whether self-testing is enabled or not for the kernel. If the kernel is built in release mode, this option is forced to `false`.
- **USERSPACE_TEST** (default: `false`): Tells if the libraries should be compiled for userspace testing purpose.
- **QEMU_TEST** (default: `false`): Tells whether the kernel should be compiled to be tested on QEMU.



### Makefile

The command `make` or `make maestro` build the kernel.
`make iso` or `make maestro.iso` build a test ISO image.

Typing `make clean` removes every object files and keeps only the original source code and the generated binaries.
Typing `make fclean` does the same as `make clean` but also removes the binaries.
And `make re` cleans and recompiles everything.



## Running

The generated ISO file can be used to run the kernel on an emulator.
`make test` allows to run it on QEMU and `make bochs` allows to run it on Bochs.
