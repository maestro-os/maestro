# Compilation

## Configuration

The configuration file is mandatory to build the kernel. It specifies which features are enabled.

A default configuration is available in the file `default.config.toml`. To use it, simply type the command:

```sh
cp default.config.toml config.toml
```



## Build

After creating the configuration, the kernel can be built using the following commands:

```sh
cargo build               # Debug mode
cargo build --release     # Release mode
```

The default architecture is `x86`. To specify an other architecture, add the following parameter to the build command: `--target arch/<arch>/<arch>.json`, where `<arch>` is the selected architecture.

The list of available architecture can be retrieved by typing:

```
ls -1 arch/
```
