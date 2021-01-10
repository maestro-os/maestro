#!/bin/bash

# This script allows to test the kernel's sub-libraries in userspace.
# It is required in order to run this script that the configuration to make Cargo build the core/std libraries itself is disabled.

libs="mem_alloc,util"

echo $libs | tr ',' '\n' | while read lib; do
	cd $lib;
	RUSTFLAGS='--cfg kernel_mode="debug" --cfg userspace -Zmacro-backtrace' cargo test;
	cd ..;
done
