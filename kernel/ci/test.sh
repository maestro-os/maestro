#!/bin/sh

set -e

export QEMUFLAGS="-nographic -monitor none -serial stdio"

cp default.build-config.toml build-config.toml
sed -i 's/^qemu = false$/qemu = true/' build-config.toml

case $1 in
	self)
		cargo test --lib
		;;
	int)
	  set +e
		cargo run
		STATUS=$?
		set -e
		fsck.ext2 -fn qemu_disk
    exit $STATUS
		;;
	*)
		>&2 echo "Invalid tests kind"
		exit 1
		;;
esac
