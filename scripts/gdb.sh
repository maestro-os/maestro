#!/bin/sh

# This script allows to run gdb to debug the kernel using QEMU.

# Environment variables:
# - ARCH: specifies the architecture to build for
# - AUX_ELF: specifies the path to an auxiliary ELF file whose symbols will be added to gdb

export QEMUFLAGS="-s -S -d int"
setsid cargo run &
QEMU_PID=$!

if [ -z "$ARCH" ]; then
  ARCH="x86"
fi
KERN_PATH="target/$ARCH/debug/maestro"

if ! [ -z "$AUX_ELF" ]; then
	gdb $KERN_PATH -ex 'target remote :1234' -ex 'set confirm off' -ex 'add-symbol-file -o $AUX_ELF' -ex 'set confirm on'
else
	gdb $KERN_PATH -ex 'target remote :1234'
fi

kill -- -$QEMU_PID
