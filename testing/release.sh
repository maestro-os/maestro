#!/bin/bash

set -e

echo "Testing release compilation..."
cp testing/configs/release .config
make maestro
stat maestro
testing/multiboot_test.sh
