# Maestro

Maestro is a lightweight Unix-like kernel written in Rust.

This repository is not a full operating system in itself but only the kernel. To build the an operating system with it, check the documentation.

![Continuous Integration](https://github.com/llenotre/maestro/actions/workflows/check.yml/badge.svg)
![Rust Version](https://img.shields.io/badge/rust-nightly_2023--05--11-lightgrey.svg)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)



## Compilation

### Configuration

The configuration allows to easily specify which features have to be enabled in the kernel. This configuration is *required* to compile the kernel.

A default configuration is available in the file `default.config.toml`. To use it, simply type the command:

```sh
cp default.config.toml config.toml
```



### Build

After creating the configuration, the kernel can be built using the following commands:

```sh
cargo build               # Debug mode
cargo build --release     # Release mode
```

The default architecture is `x86`. To specify an other architecture, add the following parameter to the build command: `--target arch/<arch>/<arch>.json`, where `<arch>` is the selected architecture.

The list of available architecture can be retrieved by typing:

```
ls -1 arch/
```



### Run

If QEMU is present on the system, the kernel can be run using:

```sh
cargo run               # Debug mode
cargo run --release     # Release mode
```

Don't forget to use the `--target` parameter if necessary.

However, running the kernel correctly requires a fully built operating system.



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
