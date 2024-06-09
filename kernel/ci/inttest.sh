#!/bin/sh

set -e

export QEMUFLAGS="-nographic -serial none -serial file:serial.log -device isa-debug-exit,iobase=0xf4,iosize=0x04"

cp default.build-config.toml build-config.toml
sed -i 's/^qemu = false$/qemu = true/' build-config.toml

rm -f serial.log
mkfifo serial.log



echo "Running integration tests..."

setsid cargo run --release &
QEMU_PID=$!

# TODO get and analyze output
