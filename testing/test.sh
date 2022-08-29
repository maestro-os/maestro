#!/bin/bash

# This script tests the compilation of the kernel. It must be run from the root of the project

status=0

# Executes the given command
exec_command () {
	echo "$> $@"
	if ! $@; then
		status=1
	fi
	echo
}

# Prints the testing environement
print_env () {
	echo "--------------------------"
	echo "   Testing environement   "
	echo "--------------------------"
	echo

	exec_command date
	exec_command uname -a
	exec_command pwd
	exec_command env
}

# Tests compilation with the current environement
test_compilation () {
	exec_command make maestro
	exec_command stat maestro
	exec_command testing/multiboot_test.sh
	exec_command make fclean
}

print_env

echo "Cleaning up..."
exec_command make fclean



echo "Checking coding style..."
exec_command testing/codecheck.sh



echo "Testing debug compilation..."
cp testing/configs/debug .config
test_compilation



echo "Testing release compilation..."
cp testing/configs/release .config
test_compilation



echo "Selftesting compilation..."
cp testing/configs/selftest .config
test_compilation

echo "Running selftests..."
exec_command make selftest

echo "Selftests output:"
cat serial.log
grep 'No more tests to run' -- serial.log >/dev/null 2>&1
status=$?



echo
if [ "$status" = "0" ]; then
	echo "Done testing, OK :D"
else
	echo "Done testing, KO :("
fi

exit $status
