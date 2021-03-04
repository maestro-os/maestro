#!/bin/bash

# This script tests the compilation of the kernel.

set -e
unset KERNEL_ARCH
unset KERNEL_MODE
unset KERNEL_TEST
unset KERNEL_QEMU_TEST

# Prints the testing environement
print_env () {
	echo "--------------------------"
	echo "   Testing environement   "
	echo "--------------------------"
	echo

	echo "$ date"
	date
	echo

	echo "$ uname -a"
	uname -a
	echo

	echo "$ pwd"
	pwd
	echo

	echo "$ env"
	env
	echo
}

# Tests compilation with the current environement
test_compilation () {
	echo "$> make maestro"
	make maestro
	echo

	echo "$> stat maestro"
	stat maestro
	echo

	echo "$> make fclean"
	make maestro
	echo

	echo "$> ! stat maestro"
	! stat maestro
	echo
}

print_env

echo "Checking coding style..."
echo "$> ./codecheck.sh"
./codecheck.sh
echo

echo "Testing default compilation..."
test_compilation

echo "Testing debug compilation..."
export KERNEL_MODE=debug
test_compilation
unset KERNEL_MODE



echo "Testing test compilation..."
export KERNEL_MODE=debug
export KERNEL_TEST=true
test_compilation
unset KERNEL_MODE
unset KERNEL_TEST



echo "Testing release compilation..."
export KERNEL_MODE=release
test_compilation
unset KERNEL_MODE

echo "Done testing, success!"
