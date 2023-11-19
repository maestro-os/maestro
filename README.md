<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/llenotre/maestro-lnf/master/logo-light.svg">
    <img src="https://raw.githubusercontent.com/llenotre/maestro-lnf/master/logo.svg" alt="logo" width="50%" />
  </picture>
</p>

[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&logo=book)](./LICENSE)
![Version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fllenotre%2Fmaestro%2Fmaster%2FCargo.toml&query=%24.package.version&style=for-the-badge&label=version)
![Rust toolchain](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fllenotre%2Fmaestro%2Fmaster%2Frust-toolchain.toml&query=%24.toolchain.channel&style=for-the-badge&logo=rust&label=rust%20toolchain&color=%23444)
![Continuous integration](https://img.shields.io/github/actions/workflow/status/llenotre/maestro/check.yml?style=for-the-badge&logo=github)
![Stars](https://img.shields.io/github/stars/llenotre/maestro?style=for-the-badge&color=yellow)
[![Discord](https://img.shields.io/discord/971452040821760080?style=for-the-badge&logo=discord&color=%235865f2)](https://discord.gg/4JMBN3YPAk)



# About

Maestro is a lightweight Unix-like kernel written in Rust.

The goal is to provide a lightweight operating system able to use the safety features of the Rust language to be reliable.

> This project is still in early stage development, thus it is highly unstable and misses a lot of features. **Do not use it in production!**

To stay updated with the project, follow the [blog](https://blog.lenot.re)!

<p align="center">
  <img src="https://blog.lenot.re/assets/article/neofetch.png" alt="neofetch" width="100%" />
</p>

[Neofetch](https://github.com/dylanaraps/neofetch) and bash running on the OS.



# Features

The following features are currently implemented (non-exhaustive):
- Terminal with [VGA text mode](https://en.wikipedia.org/wiki/VGA_text_mode) and [PS/2](https://en.wikipedia.org/wiki/PS/2_port) keyboard (with forward compatibility with USB handled by the motherboard's firmware)
    - Partial support of [ANSI escape codes](https://en.wikipedia.org/wiki/ANSI_escape_code)
- Memory allocation/virtual memory
    - [Buddy allocator](https://en.wikipedia.org/wiki/Buddy_memory_allocation)
    - Internal memory allocator, with similarities with **dlmalloc**'s implementation, working on top of the buddy allocator
- Processes and [scheduler](https://en.wikipedia.org/wiki/Scheduling_(computing)) ([round-robin](https://en.wikipedia.org/wiki/Round-robin_scheduling))
    - POSIX signals
- [PCI](https://en.wikipedia.org/wiki/Peripheral_Component_Interconnect) devices enumeration
- Files:
    - Mountpoints
    - [IDE/PATA](https://en.wikipedia.org/wiki/Parallel_ATA) driver
    - Filesystem ([ext2](https://en.wikipedia.org/wiki/Extended_file_system) only for now)
    - Disk partitions ([MBR](https://en.wikipedia.org/wiki/Master_boot_record) and [GPT](https://en.wikipedia.org/wiki/GUID_Partition_Table))
    - Virtual filesystems (`/tmp` and `/proc`)
    - initramfs
- Time/Clock ([RTC](https://en.wikipedia.org/wiki/Real-time_clock))
- Linux system calls (roughly 30% are currently implemented)
- Kernel modules
- [ELF](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format) programs



# Quickstart

This repository is not a full operating system in itself but only the kernel.

You can either:
- Use the [installer](https://github.com/llenotre/maestro-install) to build a full operating system from an ISO file
- Build the OS by hand. For this, you can check the kernel's book

The OS can then be run by a virtual machine such a **QEMU** or **VirtualBox**, or on a physical machine.



## Build

The configuration allows to easily specify which features have to be enabled in the kernel. This configuration is *required* to compile the kernel.

A default configuration is available in the file `default.config.toml`. To use it, simply type the command:

```sh
cp default.config.toml config.toml
```

After configuration, the kernel can be built using the following commands:

```sh
cargo build               # Debug mode
cargo build --release     # Release mode
```

The default architecture is `x86`. To specify another architecture, add the following parameter to the build command: `--target arch/<arch>/<arch>.json`, where `<arch>` is the selected architecture.

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

### The book

The kernel's book contains general information on how to use the kernel.

The book can be built using *mdbook*, with the command:

```sh
mdbook build doc/
```

Then, it can be accessed at `doc/book/index.html`.



### References

The references contain the documentation for functions, structures, etc...

It can be built using the command:

```sh
cargo doc
```

Then, it can be accessed at `target/<arch>/doc/kernel/index.html`, where `<arch>` is the architecture the kernel has been compiled for.
