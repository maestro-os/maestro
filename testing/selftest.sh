#!/bin/bash

set -e

echo "Selftest compilation..."
cp testing/configs/selftest .config
make maestro
stat maestro
testing/multiboot_test.sh

echo "Running selftests..."
make selftest

echo "Selftests output:"
cat serial.log
grep 'No more tests to run' -- serial.log >/dev/null 2>&1
