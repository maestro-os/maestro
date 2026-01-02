# Kernel modules

Kernel modules add features to the kernel at runtime. They are especially useful for implementing drivers.

A kernel module has the same privileges as the kernel itself and runs in the same memory space. As such, one must be careful when trusting a kernel module.

From the point of view of the kernel, the module is a shared library (`.so`) that is loaded pretty much like a regular one.
The kernel relocates the module against itself at load time.

At build time, a kernel module is tricked into thinking the kernel is also a shared library. This is necessary to prevent linking the whole kernel inside each module.

Of course, at runtime the kernel is a normal ELF executable (GRUB does not support relocating the kernel's ELF anyway).

## Kernel module template

A kernel module template is available in `mod/template/`. It has the following files:

```
|- Cargo.toml
|- Cargo.lock
|- src/
 |- mod.rs
```

`Cargo.toml`:

```toml
{{#include ../../../mod/template/Cargo.toml}}
```

`mod.rs`:

```rust
{{#include ../../../mod/template/src/mod.rs}}
```

The `kernel` crate gives access to the kernel's functions.

The `kernel::module` macro allows to define the kernel module with its dependencies.

> **NOTE**: if the `kernel::module` declaration is not present, the module will not work

The following properties have to be taken into account when writing a module:
- `init` is called once each times the module is loaded. The execution must be not block since it would freeze the system
- `fini` can be called at all times and must free every resource allocated by the module

On success, `init` returns `true`. On failure, it returns `false`.

## In-tree modules

It is recommended (although not mandatory) to keep kernel modules inside the kernel's repository. As such, they can be maintained with the rest of the kernel.

In-tree modules are located in the `mod/` directory.

> **NOTE**: if a module is maintained out of tree, it is important to ensure it has an up-to-date `rust-toolchain.toml`, such as the version of the Rust toolchain is the same as the kernel (see `rust-toolchain.toml` at the root of the kernel's repository).

## Versioning

Kernel module versioning is a small subset of the [SemVer](https://semver.org/) specification.

Versions MUST have the following format: `X.Y.Z` where:
- `X` is a positive number (including zero) representing the *major version*
- `Y` is a positive number (including zero) representing the *minor version*
- `Z` is a positive number (including zero) representing the *patch version*

The same rules as the SemVer specification apply for those numbers.

### Backus-Naur Form

```
<version> ::= <major> "." <minor> "." <patch>
```

## Interface references

The references to the kernel's internals and module interfaces can be found [here](references/kernel/index.html).

## Building

The procedure to build a kernel module is the following:
- Build the kernel
- `cd` into the root of the module's root directory (containing the module's `Cargo.toml`)
- Set (optional) environment variables:
    - `ARCH`: architecture to build for (default: `x86_64`)
    - `CMD`: the cargo command to use (default: `build`)
    - `PROFILE`: the profile to build for. This is usually `debug` or `release` (default: `debug`)
- Build the module

Example:
```sh
ARCH="x86" PROFILE="debug" ../build
```

Then, the built module can be found at `target/<arch>/<profile>/lib<name>.so`

> **NOTE**: It is important that the specified profile and architecture match the compiled kernel's, otherwise compilation will not work
