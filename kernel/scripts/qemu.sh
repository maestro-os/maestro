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
GRUB_MKRESCUE=grub-mkrescue

# Check if a program exists
# $1 => program name
check_program() {
	if ! command -v "${1}" &>/dev/null
	then
		>&2 echo "${1} could not be found."
		exit 1
	fi
}

# Check that all dependencies are installed
check_program $QEMU
check_program $GRUB_MKRESCUE

# Build ISO

mkdir -p iso/boot/grub
cp $1 iso/boot/maestro
cp grub.cfg iso/boot/grub
${GRUB_MKRESCUE} -o kernel.iso iso

# Run the kernel

export QEMUDISK=qemu_disk
export QEMUFLAGS="-device isa-debug-exit,iobase=0xf4,iosize=0x04 $QEMUFLAGS"
if [ -f $QEMUDISK ]; then
  QEMUFLAGS="-drive file=$QEMUDISK,format=raw $QEMUFLAGS"
fi

${QEMU} -cdrom kernel.iso $QEMUFLAGS
EXIT=$?

if [ "$EXIT" -ne 33 ]; then
	exit 1
fi
