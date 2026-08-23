#!/bin/sh

# This script builds the blimp package manager.

set -e

REPO="${BLIMP_REPO:-https://github.com/maestro-os/blimp}"
REF="${BLIMP_REF:-master}"

OUT="${1:-blimp-bin}"
mkdir -p "$OUT"
OUT=$(realpath "$OUT")

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

git clone --depth 1 --branch "$REF" "$REPO" "$WORK"
cd "$WORK"
cargo build --release --package blimp

cp target/release/blimp "$OUT/"
