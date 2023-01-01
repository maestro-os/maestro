#!/bin/bash

# This script allows to run gdb to debug the kernel using QEMU.

# The AUX_ELF environment variable allows to specify the path to an auxilary ELF file whoses symbols will be added to gdb.
# This allows to debug the kernel with a given running program.

setsid make debug &
QEMU_PID=$!

if [ "$AUX_ELF" != "" ]; then
	gdb maestro -ex 'target remote :1234' -ex 'set confirm off' -ex "add-symbol-file -o 0x19c000 $AUX_ELF" -ex 'set confirm on'
else
	gdb maestro -ex 'target remote :1234'
fi

kill -- -$QEMU_PID
