#!/bin/bash

# This script is meant to automatically check for code style errors in the source code.

status=0

# Checks for lines that has a space in its indentation (except documentation comments)
echo "Checking for spaces in indentation..."
if grep '^	* [^\*]' -rn src/; then
	status=1
fi
echo

# Checks for lines that end with a space
echo "Checking for lines ending with a space..."
if grep ' $' -rn src/; then
	status=1
fi
echo

# Checks for lines that end with a tab
echo "Checking for lines ending with a tab..."
if grep '	$' -rn src/; then
	status=1
fi
echo

# Checks for lines in C code longer than 80 characters
echo "Checking for lines of C code longer than 80 characters..."
if grep '.\{81,\}' -rn $(find src/ -name "*.[ch]"); then
	status=1
fi
echo

# Checks for lines in Makefiles longer than 120 characters
echo "Checking for lines of Makefile longer than 120 characters..."
if grep '.\{121,\}' -rn Makefile; then
	status=1
fi
echo

# Checks for lines in Rust code longer than 99 characters
echo "Checking for lines of Rust code longer than 99 characters..."
if grep '.\{100,\}' -rn $(find src/ -name "*.rs"); then
	status=1
fi
echo

# TODO Check documentation at the beginning of files

exit $status
