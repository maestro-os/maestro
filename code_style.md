# Code style

The kernel's coding style allows to get uniform code syntax into the whole project.
This document aims to describes the rules that must be respected.

The code style can be check by running the script `codecheck.sh`.
The script must return an exit code of `0` to consider that the code style is valid.



## Line length

The length of a line **must not** exceed the following length:
- C language: 80 characters
- Makefile: 120 characters
- Rust language: 99 characters



## Indentation

The indentation **must** be composed of tabulations only.
A line **must not** begin with a whitespace, except for documentation comments.



## Junk at the end of line

Lines **must not** end with whitespace or tabulations.



## Documentation

The following elements **must** be documented:
- Constant
- Enum
- Function
- Macro
- Static Variable
- Structure
- Structure field
- Type definition
- Union

Files themselves **must** also begin with a documentation comment describing the role of the module.
Makefile variables and rules **must** also be documented.

A documentation comment for **C** and **Rust** code shall have the following syntax:

```
/*
 * Here is the documentation
 */
```

The first line **must not** begin with a whitespace nor contain any documentation text.
The following lines **must** begin with one, and only one whitespace.
Lines that contain documentation shall begin with exactly ` * `. Several of these lines can be added one after the other.

Example:

```
/*
 * Documentation 0
 * Documentation 1
 * Documentation 2
 * ...
 */
```
