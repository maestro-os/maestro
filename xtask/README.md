# xtask

Build orchestrator for the Maestro kernel, following the [cargo-xtask](https://github.com/matklad/cargo-xtask) convention.

Run from the **repository root** via the `cargo xtask` alias defined in `.cargo/config.toml`.

## Commands

### `build-kernel`

```sh
cargo xtask build-kernel              # debug
cargo xtask build-kernel --release    # release
cargo xtask build-kernel --target arch/x86/x86.json
```

Builds the kernel, automatically embedding any built-in modules configured in
`kernel/build-config.toml` under `[modules] builtin = [...]`.

#### Why two passes?

Module compilation requires `libkernel.rlib` (modules link against the kernel).
That file is produced by the kernel build itself, so it doesn't exist yet when
`build.rs` runs for the first time. The `build-kernel` command handles this by
running `cargo build` inside `kernel/` twice:

1. **First pass** — compiles the kernel; `build.rs` skips module embedding because
   `libkernel.rlib` is not yet present.
2. **Second pass** — `cargo:rerun-if-changed=libkernel.rlib` triggers `build.rs`
   again; modules are compiled against the now-available rlib and embedded into the
   binary.

When no built-in modules are configured the second pass is a fast no-op (Cargo
detects nothing changed).

All flags (`--release`, `--target`, etc.) are forwarded verbatim to both `cargo
build` invocations.
