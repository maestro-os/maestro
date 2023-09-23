#!/bin/bash



# Build ISO
mkdir -p iso/boot/grub
cp $1 iso/boot/maestro
cp grub.cfg iso/boot/grub
grub-mkrescue -o kernel.iso iso



# Run the kernel

export QEMU_DISK=qemu_disk
export QEMUFLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMUFLAGS"

if [ -f $QEMU_DISK ]; then
	QEMUFLAGS="-drive file=$QEMU_DISK,format=raw $QEMUFLAGS"
fi

qemu-system-i386 -cdrom kernel.iso $QEMUFLAGS >qemu.log 2>&1
