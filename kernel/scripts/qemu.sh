#!/bin/sh

# Runs the kernel in QEMU. This script is meant to be used through `cargo`

if [ -z "$ARCH" ]; then
  ARCH="x86_64"
fi

case $ARCH in
	"x86")
		QEMU=qemu-system-i386
		;;
	"x86_64")
		QEMU=qemu-system-x86_64
		;;
	*)
		>&2 echo "Invalid architecture '$ARCH'"
		exit 1
		;;
esac

# Default to NVMe
if [ -z $DISKTYPE ]; then
	DISKTYPE="nvme"
fi
case $DISKTYPE in
	ide)
		DISKMAJOR=8
		;;
	nvme)
		DISKMAJOR=259
		;;
	*)
		>&2 echo "Invalid disk type '$DISKTYPE'"
		exit 1
		;;
esac

# Build ISO

mkdir -p iso/boot/grub
cp $1 iso/boot/maestro
cp grub.cfg iso/boot/grub
sed -i "s/ROOTMAJOR/$DISKMAJOR/" iso/boot/grub/grub.cfg
grub-mkrescue -o kernel.iso iso

# Run the kernel

export QEMUDISK=qemu_disk
export QEMUFLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMUFLAGS"
if [ -f $QEMUDISK ]; then
	case $DISKTYPE in
		ide)
			QEMUFLAGS="-drive file=$QEMUDISK,format=raw $QEMUFLAGS"
			;;
		nvme)
			QEMUFLAGS="-device nvme,serial=deadbeef,drive=nvme -drive file=$QEMUDISK,format=raw,if=none,id=nvme $QEMUFLAGS"
			;;
	esac
fi

${QEMU} -cdrom kernel.iso $QEMUFLAGS
EXIT=$?

if [ "$EXIT" -ne 33 ]; then
	exit 1
fi
