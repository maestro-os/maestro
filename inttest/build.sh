#!/bin/sh

set -e

if [ -z "$TARGET" ]; then
	export TARGET=x86_64-unknown-linux-musl
fi
if [ -z "$ARCH" ]; then
	case ${TARGET%%-*} in
		i*86)
			ARCH=x86
			;;
		x86_64)
			ARCH=x86_64
			;;
		*)
			>&2 echo "Unsupported architecture"
			exit 1
			;;
	esac
	export ARCH
fi
case ${TARGET%%-*} in
	i*86)
		PKG_ARCH=x86
		;;
	*)
		PKG_ARCH=${TARGET%%-*}
		;;
esac

if ! command -v blimp >/dev/null; then
	>&2 echo "\`blimp\` not found in PATH"
	exit 1
fi

# Build programs
cargo build -Zbuild-std --target "$TARGET"
# Build kernel module
cd mod/
cargo clean
../../mod/build
cd ..

# Populate the system root
ROOT=root/
rm -rf "$ROOT"
mkdir -p "$ROOT/"{dev,sbin,var/lib/blimp}
cp "target/$TARGET/debug/init" "$ROOT/sbin/init"
cp "target/$TARGET/debug/inttest" "$ROOT/inttest"
cp "mod/target/$ARCH/debug/libinttest.so" "$ROOT/mod.kmod"
cp test/hello.{c,cpp} "$ROOT"
echo "pkg.maestro-os.org" >"$ROOT/var/lib/blimp/remotes-list"
# Install packages
SYSROOT="$ROOT" blimp update
yes | SYSROOT="$ROOT" blimp --arch "$PKG_ARCH" install binutils coreutils gcc musl

# Create disk and filesystem
rm -f disk
dd if=/dev/zero of=disk bs=1M count=4096
fakeroot -- sh -c "chown -R 0:0 '$ROOT' && mkfs.ext2 -d '$ROOT' disk"
