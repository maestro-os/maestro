# util

Utilities are implemented in a separate library to allow easier debugging, by running tests in userspace.



## Run tests

You can use the command:

```sh
cargo test
```

to run unit tests in userspace.

Tests can also be run with [Miri](https://github.com/rust-lang/miri) using:

```sh
cargo miri test
```