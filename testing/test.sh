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

	echo "-------------"
	echo "   Sources   "
	echo "-------------"
	echo

	exec_command ls -R .
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



echo "Selftesting compilation..."
cp testing/configs/selftest .config
test_compilation



#echo "Running selftests..."
#make selftest
#exec_command serial.log
# TODO Check if the results are correct



echo "Testing release compilation..."
cp testing/configs/release .config
test_compilation



echo
if [ "$status" = "0" ]; then
	echo "Done testing, OK :D"
else
	echo "Done testing, KO :("
fi

exit $status
