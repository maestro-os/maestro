#!/bin/bash



# Build ISO
mkdir -p iso/boot/grub
cp target/$ARCH/$PROFILE/kernel iso/boot/maestro
cp grub.cfg iso/boot/grub
grub-mkrescue -o kernel.iso iso



# Run the kernel

export QEMU_DISK=qemu_disk
QEMU_FLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMU_FLAGS"

if [ -f $QEMU_DISK ]; then
	QEMU_FLAGS="-drive file=$QEMU_DISK,format=raw $QEMU_FLAGS"
fi

qemu-system-i386 -cdrom kernel.iso $QEMU_FLAGS >qemu.log 2>&1
