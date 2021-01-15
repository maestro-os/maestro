#!/bin/bash

# This script allows to test the kernel's sub-libraries in userspace.
# It is required in order to run this script that the configuration to make Cargo build the core/std libraries itself is disabled.

libs="mem_alloc,util"
target_file=$(pwd)/arch/x86/target.json

echo $libs | tr ',' '\n' | while read lib; do
	make $lib/lib${lib}.a;
done || exit 1

export USERSPACE_TEST=true
export RUSTFLAGS='-Zmacro-backtrace --cfg kernel_mode="debug" --cfg userspace'

echo $libs | tr ',' '\n' | while read lib; do
	cd $lib;
	cargo +nightly test --verbose --target $target_file;
	cd ..;
done
