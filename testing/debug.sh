#!/bin/bash

set -e

echo "Testing debug compilation..."
cp testing/configs/debug .config
make maestro
stat maestro
testing/multiboot_test.sh
