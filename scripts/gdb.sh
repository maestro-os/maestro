#!/bin/bash

# This script allows to run gdb to debug the kernel using QEMU.

# The AUX_ELF environment variable allows to specify the path to an auxilary ELF file whoses symbols will be added to gdb.
# This allows to debug the kernel with a given running program.

export QEMU_FLAGS="-s -S -d int"
setsid scripts/qemu.sh &
QEMU_PID=$!

export KERN_PATH="target/$ARCH/debug/kernel"

if ! [ -z "$AUX_ELF" ]; then
	gdb $KERN_PATH -ex 'target remote :1234' -ex 'set confirm off' -ex 'add-symbol-file -o 0x19c000 $AUX_ELF' -ex 'set confirm on'
else
	gdb $KERN_PATH -ex 'target remote :1234'
fi

kill -- -$QEMU_PID
