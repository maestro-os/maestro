# Maestro

Maestro is a simple OS created just for fun.



## Compilation

To compile this project, you must compile a cross compiler, following the instructions on this page: [GCC Cross Compiler](https://wiki.osdev.org/GCC_Cross-Compiler)
GRUB is also needed in order to boot the kernel.

Typing `make` will generate a binary and an ISO file.



### Environement variables

Some environement variables can be used to customize the compilation of the kernel. The list is the following:
- **KERNEL_ARCH** (default: `x86`): Specify the platform for which the kernel will be compiled. The list of available platform is located in the `arch/` directory.
- **KERNEL_MODE** (default: `debug`): The mode of the kernel. Either `debug` or `release`.
- **KERNEL_TEST** (default: `false`): Tells whether self-testing is enabled or not for the kernel. If the kernel is built in release mode, this option is forced to `false`.



## Running

The generated ISO file can be used to run the kernel on an emulator.
`make test` allows to run it on QEMU and `make bochs` allows to run it on Bochs.
