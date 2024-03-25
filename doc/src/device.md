# Devices

Devices are represented by two types of files: Block Devices and Char Devices.

Each device file is also associated with a major and minor number, allowing to identify it.

Those files are usually present in the `/dev` directory.

Device type abbreviations:
- C = Char Device
- B = Block Device

The following sections describe devices that may be present on the system. This list may be extended by kernel modules and as such, doesn't include every possible devices.



## Default devices list

The following devices are present on the system by default.

| Path           | Type | Major | Minor | Description                                                                                                                             |
|----------------|------|-------|-------|-----------------------------------------------------------------------------------------------------------------------------------------|
| `/dev/null`    | C    | `1`   | `3`   | This device does nothing. Reading from it returns EOF and writing to it discards the data                                               |
| `/dev/zero`    | C    | `1`   | `5`   | Reading returns an infinite amount of zeros bytes and writing to it discards the data                                                   |
| `/dev/random`  | C    | `1`   | `8`   | Reading returns random bytes and writing to it feeds the kernel's entropy pool. If not enough entropy is available, reading is blocking |
| `/dev/urandom` | C    | `1`   | `9`   | Reading returns random bytes and writing to it feeds the kernel's entropy pool. Contrary to `/dev/random`, reading is never blocking    |
| `/dev/kmsg`    | C    | `1`   | `11`  | Reading returns kernel logs and writing appends kernel logs                                                                             |
| `/dev/tty`     | C    | `5`   | `0`   | Device representing the TTY of the current process                                                                                      |



## Dynamic devices

This section describes devices that may or may not be present depending on the system's peripherals.

| Path        | Type | Major | Minor            | Description                                                                                                                                                                 |
|-------------|------|-------|------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `/dev/sdX`  | B    | `8`   | `n * 16`         | A SCSI drive. `X` has to be replaced by a single letter. Each disk has its own unique letter. `n` is the number associated with the letter (`a` -> `0`, `b` -> `1`, etc...) |
| `/dev/sdXN` | B    | `8`   | `n * 16 + N + 1` | A partition on a SCSI drive. This device works the same as the previous, except `N` is the partition number                                                                 |
