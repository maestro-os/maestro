#!/bin/bash

export QEMU_FLAGS="-serial file:serial.log"

cp default.config.toml config.toml
sed -i 's/^qemu = false$/qemu = true/' config.toml

echo "Running selftests..."
cargo run

echo "Selftests output:"
cat serial.log
grep 'No more tests to run' -- serial.log >/dev/null 2>&1
