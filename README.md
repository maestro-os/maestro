# Maestro

Maestro is a lightweight Unix-like kernel written in Rust.



## Compilation

The first step in compiling the kernel is typing `make config`, which compiles and opens a configuration utility, allowing to create the configuration file.

Next, typing `make` shall compile everything (including documentation).



## Booting

The kernel uses Multiboot2 to boot, thus it requires a bootloader compatible with it such as GRUB2.

A command line argument is required to tell which device is to be used as the VFS's root (see the documentation for more informations).



## Documentation

The kernel's internal documentation can be compiled using the `make doc` command. It contains a description of the kernel's internal workings and code references.
