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



Configuration
=============

Before building the kernel, some environement variables can be set in order to configure the compilation of the kernel.
When changing the environement variables, it's highly recommended to recompile everything.

Here is the list of environement variables:

+------------------+---------------+-----------------------------------------------------------------------------------------------------------------+
| Name             | Default value | Description                                                                                                     |
+==================+===============+=================================================================================================================+
| KERNEL_ARCH      | ``x86``       | Specifies the platform for which the kernel will be compiled                                                    |
+------------------+---------------+-----------------------------------------------------------------------------------------------------------------+
| KERNEL_MODE      | ``debug``     | Enables the debug mode of the kernel (on by the default). Possible values are either `debug` or `release`       |
+------------------+---------------+-----------------------------------------------------------------------------------------------------------------+
| KERNEL_TEST      | ``false``     | Tells whether self-testing is enabled. If the kernel is built in release mode, this option is forced to `false` |
+------------------+---------------+-----------------------------------------------------------------------------------------------------------------+
| KERNEL_QEMU_TEST | ``false``     | Tells whether the kernel should be compiled to be tested on QEMU                                                |
+------------------+---------------+-----------------------------------------------------------------------------------------------------------------+



Kernel compilation
==================

Once the compiler is ready, the kernel can be compiled using command "make".

.. code:: shell
	make maestro # Builds the kernel
	make maestro.iso # Builds an .iso file image
	make doc # Builds the documentation
	make all # Builds everything

The default rule is "all".

The following Makefile rules are also available:

.. code:: shell
	make clean # Clean the project directory
	make fclean # Cleans the project directory and removes executable files and documentation
	make re # Remakes the project (equivalent to: make fclean; make all)
	make tags # Creates a "tags" file
	make test # Runs QEMU with the kernel image
	make cputest # Runs QEMU with the kernel image and dumps CPU state and interruptions into the file "cpu_out"
	make bochs # Runs Bochs with the kernel image using the configuration in the project's directory
	make virtualbox # Runs Virtualbox
