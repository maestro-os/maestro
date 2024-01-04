# Kernel

## Dependencies

Building the kernel requires the following dependencies:

- A i386 toolchain (espicially `i386-elf-ld`)
- `grub`, to create the bootable disk image
- `libisoburn`, needed to create the iso
- `mtools`
- `qemu` to run it in QEMU (`cargo run`)



## Configuration

> This section is optional. By default, the kernel will use the configuration in `default.build-config.toml`

Before building, one can optionally configure the kernel by copying `default.build-config.toml` into `build-config.toml`:

```sh
cp default.build-config.toml build-config.toml
```

The configuration specifies features to be enabled/disabled in the kernel.



## Build

The kernel can be built using the following commands:

```sh
cargo build               # Debug mode
cargo build --release     # Release mode
```

The default architecture is `x86_64`. To specify another architecture, add the following parameter to the build command: `--target arch/<arch>/<arch>.json`, where `<arch>` is the selected architecture.

The list of available architectures can be retrieved by typing the command:

```sh
ls -1 arch/
```



## Run

### With QEMU

QEMU is the preferred virtual machine to test the kernel.

To install QEMU, type the following command:

Ubuntu/Debian:

```sh
apt install qemu
```

Arch Linux:

```sh
pacman -S qemu
```

A fully built operating system is required to run the system. This system must be present on a raw disk in the file `qemu_disk` at the root of the repository. The option `-drive file=qemu_disk,format=raw` is used on QEMU to reference the disk.

The kernel can be run using:

```sh
cargo run               # Debug mode
cargo run --release     # Release mode
```


#### Run unit tests

The following command runs unit tests in QEMU:

```sh
cargo test --lib
```



## Documentation

The documentation of the kernel's interface for modules can be built using:

```sh
cargo doc
```

Then, it can be accessed at `target/<arch>/doc/kernel/index.html`, where `<arch>` is the architecture the kernel has been compiled for.
