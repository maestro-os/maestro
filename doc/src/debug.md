# Debug

This section describes debugging features integrated to the kernel.



## Selftesting

Unit tests and integration tests are present in the kernel.

To run them, use the command:

```sh
cargo test --lib`
```



## GDB

GDB can be attached to the kernel in order to debug it. To do so, run the script located at `scripts/gdb.sh`.

The script runs the kernel with QEMU, using the disk present in the file `qemu_disk` and automaticaly attaches GDB to it. To begin execution, just type the `continue` command on GDB.



## Logging

The kernel can transmit logs to another machine (the host machine if running in a virtual machine) using the serial port.

On QEMU, logs can be saved to the `serial.log` file by setting the `QEMU_FLAGS` environment variable:

```
QEMU_FLAGS="-serial file:serial.log" cargo run
```
