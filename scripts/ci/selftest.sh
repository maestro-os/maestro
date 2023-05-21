#!/bin/sh

set -e

export QEMU_FLAGS="-nographic -serial file:serial.log -serial mon:null -device isa-debug-exit,iobase=0xf4,iosize=0x04"

cp default.config.toml config.toml
sed -i 's/^qemu = false$/qemu = true/' config.toml

rm -f serial.log



echo "Running selftests..."

set +e
cargo test --lib
EXIT=$?
set -e



echo
echo "Selftests output:"
cat serial.log

if [ "$EXIT" -ne 33 ]; then
	exit 1
fi
