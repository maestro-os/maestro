# Debug

This section describes debugging features integrated to the kernel.



## Selftesting

The kernel can be configured to be compiled with unit tests. These tests are run at boot and check the kernel's internal functions.

To enable them, check the configuration utility.



## GDB

GDB can be attached to the kernel in order to debug it. To do so, run the script located at `scripts/debug.sh`.

The script runs the kernel in QEMU and automaticaly attaches GDB to it. To begin execution, just type the `continue` command.
