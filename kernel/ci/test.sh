#!/bin/sh

set -e

export QEMUFLAGS="-nographic -serial none -serial file:stdio -device isa-debug-exit,iobase=0xf4,iosize=0x04"

cp default.build-config.toml build-config.toml
sed -i 's/^qemu = false$/qemu = true/' build-config.toml

case $1 in
	self)
		cargo test --lib
		;;
	int)
		cargo run
		;;
	*)
		>&2 echo "Invalid tests kind"
		exit 1
		;;
esac
