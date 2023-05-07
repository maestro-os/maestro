#!/bin/bash

cp default.config.toml config.toml

for arch in $(ls -1 arch/); do
	echo "Build for architecture $arch..."
	cargo build --target arch/$arch/$arch.json
done
