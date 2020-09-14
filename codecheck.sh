#!/bin/bash

# Checks for lines that has a space in its indentation (except documentation comments)
grep '^	* [^\*]' -rn src/
# Checks for lines that end with a space
grep ' $' -rn src/
# Checks for lines that end with a tab
grep '	$' -rn src/

# Checks for lines in C code longer than 80 characters
grep '.\{81,\}' -rn $(find src/ -name "*.[ch]")
# Checks for lines in Makefiles longer than 120 characters
grep '.\{121,\}' -rn Makefile # TODO Includes all makefiles
# Checks for lines in Rust code longer than 99 characters
grep '.\{100,\}' -rn $(find src/ -name "*.rs")

# TODO Check documentation at the beginning of files

# TODO Return an exit code of `1` if any `grep` has an exit code of `0`
