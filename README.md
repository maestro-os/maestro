<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/llenotre/maestro-lnf/master/logo-light.svg">
    <img src="https://raw.githubusercontent.com/llenotre/maestro-lnf/master/logo.svg" alt="logo" width="50%" />
  </picture>
</p>

[![AGPL-3.0 license](https://img.shields.io/badge/license-AGPL--3.0-blue.svg?style=for-the-badge&logo=book)](./COPYING)
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

To build and/or run the OS, `cd` into the kernel's crate:

```sh
cd kernel/
```

Then follow the instructions in [README.md](kernel/README.md)



## Documentation

The kernel's book contains general information on how to use the kernel.

The book can be built using *mdbook*, with the command:

```sh
mdbook build doc/
```

Then, it can be accessed at `doc/book/index.html`.
