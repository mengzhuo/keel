#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Build the crabc-utils multicall binary for each release architecture.
#
# crabc-utils is a normal Rust program built for the musl target.  It is
# statically linked so the tools work on any Alpine/crabc system without a
# libgcc or interpreter dependency.  The commands themselves are Rust
# implementations; no musl-utils C code is used.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="${DIST:-$ROOT/dist}"

build() {
    local arch="$1" target="$2" linker="$3"
    echo "building crabc-utils for $arch"
    local envname="CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER"
    local rustflags="-C target-feature=+crt-static"
    if [ "$arch" = riscv64 ]; then
        # The riscv64 target has no crt-static-default.  Point rustc at the
        # self-contained musl sysroot and rust-lld instead of the glibc
        # cross compiler.
        local sysroot host lld
        sysroot="$("${RUSTC:-rustc}" --print sysroot)"
        host="$("${RUSTC:-rustc}" -vV | sed -n 's/^host: //p')"
        lld="$sysroot/lib/rustlib/$host/bin/rust-lld"
        linker="$lld"
        rustflags="$rustflags -C link-self-contained=yes -C linker=$lld"
    fi
    env RUSTFLAGS="$rustflags" "$envname=$linker" \
        cargo build --release -p crabc-utils --target "$target"

    local out="$DIST/utils/$arch"
    mkdir -p "$out"
    cp "target/$target/release/crabc-utils" "$out/crabc-utils"
    echo "built $out/crabc-utils"
}

build x86_64 x86_64-unknown-linux-musl musl-gcc
build aarch64 aarch64-unknown-linux-musl aarch64-linux-gnu-gcc
build riscv64 riscv64gc-unknown-linux-musl riscv64-linux-gnu-gcc
