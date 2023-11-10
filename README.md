<p align="center">
    <img src="https://raw.githubusercontent.com/llenotre/maestro-lnf/master/logo.svg" alt="logo" width="50%" />
</p>

[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&logo=book)](./LICENSE)
![Rust version](https://img.shields.io/badge/rust-nightly_2023--05--11-lightgrey.svg?style=for-the-badge&logo=rust)
![Continuous integration](https://img.shields.io/github/actions/workflow/status/llenotre/maestro/check.yml?style=for-the-badge&logo=github)
[![Discord](https://img.shields.io/discord/971452040821760080?style=for-the-badge&logo=discord)](https://discord.gg/4JMBN3YPAk)

# About

Maestro is a lightweight Unix-like kernel written in Rust.

The goal is to provide an operating system free of bloats and able to use the safety features of the Rust language to be reliable.

> This project is still in early stage development, thus it is highly unstable and misses a lot of features. **Do not use it in production!**

To stay updated with the project, follow the [blog](https://blog.lenot.re)!



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
