# Integration tests

Test suite for the kernel, made to be run by continuous integration.

The test suite reports its results through the serial port.



## Build an image

To use the test suite, one must first build a disk image.

> **Note**: Building the image requires the command `debugfs`

This can be done with the `build.sh` script:
```sh
./build.sh
```

This script produces the `disk` file, which can then be used by QEMU with the following option:
```
-drive file=disk,format=raw
```
