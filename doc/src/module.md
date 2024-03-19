# Module

A kernel module allows to add a feature to the kernel at runtime.

This chapter describes how to write a kernel module.



## Template

A basic kernel module contains the following files:

```
|- Cargo.toml
|- Cargo.lock
|- rust-toolchain.toml
|- src/
 |- mod.rs
```

These files are located in the `mod/template/` directory of the kernel's sources.

`Cargo.toml`:

```toml
{{#include ../../mod/template/Cargo.toml}}
```

`mod.rs`:

```rust
{{#include ../../mod/template/src/mod.rs}}
```

The `kernel` crate gives access to the kernel's functions.

The `kernel::module` macro allows to define the kernel module with its dependencies.

> **NOTE**: if the `kernel::module` declaration is not present, the module will not work

The following properties have to be taken into account when writing a module:
- `init` is called once each times the module is loaded. The execution must be not block since it would freeze the system
- `fini` can be called at all times and must free every resource allocated by the module

On success, `init` returns `true`. On failure, it returns `false`.



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
- Build the kernel in debug or release mode (`--release`), depending on which profile you want
- `cd` into the root of the module's source directory
- Set environment variables:
    - `PROFILE`: profile to build for (either `debug` or `release`). Default value: `debug`
    - `ARCH`: architecture to build for (example: `x86`). Default value: `x86`
- Build the module

Example:
```sh
PROFILE="debug" ARCH="x86" ../maestro/mod/build
```

Then, the built module can be found at `target/<arch>/<profile>/lib<name>.so`

> **NOTE**: It is important that the specified profile and architecture match the compilation of the kernel, otherwise compilation will not work
