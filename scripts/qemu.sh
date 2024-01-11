#!/bin/sh

# Runs the kernel in QEMU. This script is meant to be used through `cargo`

# Build ISO

mkdir -p iso/boot/grub
cp $1 iso/boot/maestro
cp grub.cfg iso/boot/grub
grub-mkrescue -o kernel.iso iso

# Run the kernel

export QEMUDISK=qemu_disk
export QEMUFLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMUFLAGS"
if [ -f $QEMUDISK ]; then
  QEMUFLAGS="-drive file=$QEMUDISK,format=raw $QEMUFLAGS"
fi

qemu-system-i386 -cdrom kernel.iso $QEMUFLAGS >qemu.log 2>&1
EXIT=$?

if [ "$EXIT" -ne 33 ]; then
	exit 1
fi
