# Tracing

The `memtrace` feature allows to trace usage of memory allocators. This is a debug feature, so it is not meant to be used in production.

To use it, compile the kernel with the `memtrace` feature:

```shell
cargo build --features memtrace
```

When run, the kernel will then write tracing data to the **COM2** serial port.

This data can then be fed to [kern-profile](https://github.com/llenotre/kern-profile) to generate a FlameGraph.

## Trace an allocator

An allocator can be traced using the `instrument_allocator` attribute macro (see kernel API reference).



## Data format

The output data is a series of samples. A sample represents an operation on the allocator.

The following operations exist:

| ID | Name    | Description                                    |
|----|---------|------------------------------------------------|
| 0  | alloc   | Allocate memory                                |
| 1  | realloc | Resize a previously allocated region of memory |
| 2  | free    | Frees a previously allocated region of memory  |

Each sample is written one after the other and has the following layout in memory:

| Offset      | Size         | Name   | Description                                          |
|-------------|--------------|--------|------------------------------------------------------|
| `0`         | `1`          | nlen   | The length of the name of the allocator              |
| `1`         | `nlen`       | name   | The name of the allocator                            |
| `nlen + 1`  | `1`          | op     | The ID of the operation (see table above)            |
| `nlen + 2`  | `8`          | ptr    | The address of the region affected by the operation  |
| `nlen + 10` | `8`          | size   | The new size of the region affected by the operation |
| `nlen + 18` | `1`          | nframe | The number of frames in the callstack                |
| `nlen + 19` | `nframe * 8` | frames | The pointers of the frames in the callstack          |
