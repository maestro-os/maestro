# Maestro

Maestro is a lightweight Unix-like kernel written in Rust.

This repository is not a full operating system in itself but only the kernel. To build the an operating system with it, check the documentation.

![Continuous Integration](https://github.com/llenotre/maestro/actions/workflows/check.yml/badge.svg)
![Rust Version](https://img.shields.io/badge/rust-nightly_2023--05--11-lightgrey.svg)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)



## Compilation

The kernel can be compiled by simply typing:

```sh
cargo build               # Debug mode
cargo build --release     # Release mode
```

If QEMU is present on the system, the kernel can be run using:

```sh
cargo run               # Debug mode
cargo run --release     # Release mode
```

However, running the kernel requires a fully built system.



## Documentation

### The book

The kernel's book contains general informations on how to use the kernel.

The book can be built using *mdbook*, with the command:

```sh
mdbook build doc/
```

Then, it can be accessed at `doc/book/index.html`.



### References

The references contains the documentation for functions, structures, etc...

It can be built using the command:

```sh
cargo doc
```

Then, it can be accessed at `target/<arch>/doc/kernel/index.html`, where `<arch>` is the architecture the kernel has been compiled for.
