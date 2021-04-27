Compilation
***********

This page contains instructions to compile GCC cross compiler for the kernel and some details about the kernel compilation itself.



Supported architectures
=======================

Here is the list of supported architectures

+--------------+------------------+-------------------+
| Architecture | C Cross Compiler | Target triplet    |
+==============+==================+===================+
| x86          | i686-elf-gcc     | i686-unknown-none |
+--------------+------------------+-------------------+

The corresponding compiler must be installed before compiling the kernel.



Configuration
=============

Before compiling the kernel, a configuration file named `.config` must be created to provide the compilation parameters.
This file can be created by running `make config`, which will compile and run a configuration utility.
The detail of each parameter is described in the utility itself.

After modifying the configuration file, you may want to clean the directory with `make clean` before compiling.



Kernel compilation
==================

The kernel can be compiled using command `make`.

.. code-block:: shell

	make maestro # Builds the kernel
	make maestro.iso # Builds an .iso file image
	make doc # Builds the documentation
	make all # Builds everything

The default rule is "all".

The following Makefile rules are also available:

.. code-block:: shell

	make clean # Clean the project directory
	make fclean # Cleans the project directory and removes executable files and documentation
	make re # Remakes the project (equivalent to: make fclean; make all)
	make tags # Creates a "tags" file
	make test # Runs QEMU with the kernel image
	make cputest # Runs QEMU with the kernel image and dumps CPU state and interruptions into the file "cpu_out"
	make bochs # Runs Bochs with the kernel image using the configuration in the project's directory
	make virtualbox # Runs Virtualbox
	make config # Creates or updates the configuration file
