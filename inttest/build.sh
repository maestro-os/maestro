#!/bin/sh

set -e

if [ -z "$TARGET" ]; then
	export TARGET=x86_64-unknown-linux-musl
fi
case ${TARGET%%-*} in
	i*86)
		ARCH=x86
		;;
	x86_64)
		ARCH=x86_64
		;;
	*)
		2> echo "Unsupported architecture"
		exit 1
		;;
esac

# Build programs
cargo build -Zbuild-std --target "$TARGET"
# Build kernel module
cd mod/
../../mod/build
cd ..

# Create disk and filesystem
dd if=/dev/zero of=disk bs=1M count=1024
mkfs.ext2 disk

# Fill filesystem
debugfs -wf - disk <<EOF
mkdir /dev
mkdir /sbin
write target/$TARGET/debug/init /sbin/init
write target/$TARGET/debug/inttest /inttest
write mod/target/$ARCH/debug/libinttest.so /mod.kmod
EOF
