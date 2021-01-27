#!/bin/bash

# This script allows to run gdb to debug the kernel using QEMU.

make || exit 1

setsid qemu-system-i386 -cdrom maestro.iso -d int -s -S >debug_out 2>&1 &
QEMU_PID=$!

gdb maestro -ex 'target remote :1234'
kill $QEMU_PID
