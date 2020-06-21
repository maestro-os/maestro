# Maestro

Maestro is a simple OS created just for fun.



## Compilation

To compile this project, you must compile a cross compiler, following the instructions on this page: [GCC Cross Compiler](https://wiki.osdev.org/GCC_Cross-Compiler)
GRUB is also needed in order to boot the kernel.

Typing `make` will generate a binary and an ISO file.



## Running

The generated ISO file can be used to run the kernel on an emulator.
`make test` allows to run it on QEMU and `make bochs` allows to run it on Bochs.
