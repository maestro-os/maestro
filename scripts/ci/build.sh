#!/bin/sh

EXIT_CODE=0

for arch in $(ls -1 arch/); do
	echo "Build for architecture $arch..."
	cargo build --target arch/$arch/$arch.json $CARGOFLAGS
	EXIT_CODE=$(($EXIT_CODE + $?))
done

exit $EXIT_CODE
