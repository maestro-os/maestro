#!/bin/sh

# This script allows to run gdb to debug the kernel using QEMU.

# The AUX_ELF environment variable allows to specify the path to an auxiliary ELF file whose symbols will be added to gdb.
# This allows to debug the kernel with a given running program.

export QEMUFLAGS="-s -S -d int"
setsid cargo run &
QEMU_PID=$!

# TODO support multiple archs
KERN_PATH="target/x86/debug/maestro"

if ! [ -z "$AUX_ELF" ]; then
	gdb $KERN_PATH -ex 'target remote :1234' -ex 'set confirm off' -ex 'add-symbol-file -o $AUX_ELF' -ex 'set confirm on'
else
	gdb $KERN_PATH -ex 'target remote :1234'
fi

kill -- -$QEMU_PID
