#!/bin/sh

set -e

export QEMUFLAGS="-nographic -monitor none -serial stdio $QEMUFLAGS"

cp default.build-config.toml build-config.toml
sed -i 's/^qemu = false$/qemu = true/' build-config.toml

case $1 in
	self)
		cargo test --lib $CARGOFLAGS
		;;
	int)
		cargo run $CARGOFLAGS

		# Check filesystem integrity
		fsck.ext2 -fnv qemu_disk
		# Check persistence
		echo 'Check `/persistent` exists'
		echo 'cat /persistent' | debugfs -f - qemu_disk 2>&1 | grep 'persistence OK'
		;;
	*)
		>&2 echo "Invalid tests kind"
		exit 1
		;;
esac
