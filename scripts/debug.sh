#!/bin/bash

# This script allows to run gdb to debug the kernel using QEMU.

# The AUX_ELF environment variable allows to specify the path to an auxilary ELF file whoses symbols will be added to gdb.
# This allows to debug the kernel with a given running program.

make maestro.iso || exit 1
make qemu_disk || exit 1

setsid qemu-system-i386 -cdrom maestro.iso -drive file=qemu_disk,format=raw -d int -s -S -serial file:serial.log >debug_out 2>&1 &
QEMU_PID=$!

if [ "$AUX_ELF" != "" ]; then
	gdb maestro -ex 'target remote :1234' -ex 'set confirm off' -ex "add-symbol-file $AUX_ELF" -ex 'set confirm on'
else
	gdb maestro -ex 'target remote :1234'
fi

kill $QEMU_PID
