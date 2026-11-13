#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Build the keel startup objects (crt1.o, Scrt1.o, crti.o, crtn.o).
#
# The objects are generated from Rust global_asm! sources in crt/; no C
# startup code or musl object files are copied.
#
# Usage: scripts/build-crt.sh [output-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/dist/crt}"
RUSTC="${RUSTC:-rustc}"

build_arch() {
    local arch="$1" target="$2"
    local dir="$OUT/$arch"
    mkdir -p "$dir"
    for file in crt1 crti crtn rcrt1; do
        "$RUSTC" --target "$target" --crate-type=lib --emit=obj -O \
            "$ROOT/crt/$file.rs" --out-dir "$dir"
    done
    cp "$dir/crt1.o" "$dir/Scrt1.o"
    echo "built $dir/crt1.o $dir/Scrt1.o $dir/rcrt1.o $dir/crti.o $dir/crtn.o"
}

build_arch x86_64 x86_64-unknown-linux-gnu
build_arch aarch64 aarch64-unknown-linux-gnu
build_arch riscv64 riscv64gc-unknown-linux-musl
