# Module

A kernel module allows to add a feature to the kernel at runtime.

This chapter describes how to write a kernel module.



## Template

A basic kernel module contains the following files:

```
|- rust-toolchain.toml
|- src/
 |- mod.rs
```

These files are located in the `mod/template/` directory of the kernel's sources.

`mod.rs` is the file that contains the main functions of the module. Example:

```rust
{{#include ../../mod/template/src/mod.rs}}
```

The `kernel` crate gives access to the kernel's functions.

The `kernel::module` macro allows to define the attributes of the module. Its arguments are:
- The name of the module
- The version of the module
- The list of dependencies of the module

The following properties have to be taken into account when writing a module:
- `init` is called once each times the module is loaded. The execution must be not block since it would freeze the system
- `fini` can be called at all times and must free every resources allocated by the module

On success, `init` returns `true`. On failure, it returns `false`.



## Interface references

The references to the kernel's internals and module interfaces can be found [here](references/kernel/index.html).



## Building

The kernel must be built in its directory in order to be able to build the module.

To build a kernel module:
- cd into the root of the module directory
- Execute the compile script located in the kernel's source, located at `mod/compile`. The script takes the name of the module as parameter. Example: `../maestro/mod/compile module_name`
- Done!
