#!/bin/sh

set -e

export QEMUFLAGS="-nographic -monitor none -serial stdio"

cp default.build-config.toml build-config.toml
sed -i 's/^qemu = false$/qemu = true/' build-config.toml

case $1 in
	self)
		cargo test --lib $CARGOFLAGS
		;;
	int)
		set +e
		cargo run $CARGOFLAGS
		STATUS=$?
		set -e
		# FIXME: the clock currently starts at the timestamp zero, which causes fsck to detect errors due to the low value for dtime
		#fsck.ext2 -fnv qemu_disk
		exit $STATUS
		;;
	*)
		>&2 echo "Invalid tests kind"
		exit 1
		;;
esac
