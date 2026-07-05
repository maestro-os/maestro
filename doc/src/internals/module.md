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

## Built-in modules

A module can be compiled directly into the kernel binary instead of loaded at runtime from a `.so` file. Built-in modules are available from the moment the kernel boots, before the filesystem is mounted. This makes them suitable for drivers that must be present early in the boot sequence (e.g. a keyboard driver needed before any disk is accessible).

### How it works

The build script compiles each configured module's source into the kernel crate using Rust's `#[path]` attribute. The module's `init()` function is called during kernel boot, and the module is registered in the kernel's module table exactly like a dynamically loaded one.

### Configuring built-in modules

Edit (or create) `kernel/build-config.toml` and add the module names to the `builtin` list under `[modules]`:

```toml
[modules]
builtin = ["ps2"]
```

The names must match the directory names under `mod/`. The next `cargo build` of the kernel will include those modules. No separate module build step is needed.

### Writing a module that supports both modes

A module compiled as a built-in is part of the kernel crate, so some attributes that are required for standalone `.so` builds must be suppressed. The `maestro_builtin` cfg flag is set by the kernel's build script whenever at least one built-in module is configured; modules use it to switch behaviour:

**Crate-level attributes** (`#![no_std]`, `#![no_main]`):

```rust
#![cfg_attr(not(maestro_builtin), no_std)]
#![cfg_attr(not(maestro_builtin), no_main)]
```

These are required for standalone builds but must be absent when the module source is compiled as part of the kernel crate.

**The `kernel` crate reference**:

```rust
#[cfg(not(maestro_builtin))]
#[no_link]
extern crate kernel;
```

In standalone mode the external `kernel` crate provides the types. In built-in mode the kernel crate exposes itself under the name `kernel` via `extern crate self as kernel` in its own root, so no explicit extern is needed.

**`#[no_mangle]` on `init` and `fini`**:

```rust
#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn init() -> bool { ... }

#[cfg_attr(not(maestro_builtin), unsafe(no_mangle))]
pub extern "C" fn fini() { ... }
```

`#[no_mangle]` is required for the dynamic loader to find the symbols in a `.so`. In built-in mode the kernel calls these functions by their fully qualified Rust names, so `#[no_mangle]` is not only unnecessary but would cause symbol conflicts when multiple modules are compiled into the same binary.

The `kernel::module!` macro already handles its own `#[no_mangle]` statics the same way, so no changes to that call are needed.

**`Cargo.toml`**: add the following to suppress warnings about the `maestro_builtin` cfg in standalone builds:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(maestro_builtin)'] }
```

The template in `mod/template/` already includes all of the above.

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

### As a standalone loadable module

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

The built module can be found at `target/<arch>/<profile>/lib<name>.so`.

> **NOTE**: It is important that the specified profile and architecture match the compiled kernel's, otherwise compilation will not work.

### As a built-in module

Add the module name to `builtin` in `kernel/build-config.toml` (see [Built-in modules](#built-in-modules)) and build the kernel normally:

```sh
cd kernel
cargo build
```

No separate module build step is required. The module is part of the kernel binary and does not produce a `.so`.
