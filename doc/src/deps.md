# Dependencies

The kernel needs some external components in order to work, here is the list.



## Mandatory

### Compilation

- `gcc`: A Cross-Compiler for the targeted architecture
- `make`: Utility to build using a Makefile
- `cargo`: Rust building utility
- `rustc`: Rust compiler

**Note**: The Nightly toolchain of Rust is required.



### Booting

- A bootloader supporting Multiboot2 (example: `GRUB2`)



## Optional

### Debugging

- `GDB`: GNU Debugger
- `QEMU`: Emulator
- `Bochs`: Emulator
- `Virtualbox`: Emulator



# Documentation

- `mdbook`: Documentation generator
