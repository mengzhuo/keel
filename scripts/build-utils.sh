#!/usr/bin/env bash
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
    # Force a static executable.  On x86_64/aarch64 this is a static musl
    # binary; the riscv64 target lacks crt-static-default and uses the
    # available cross toolchain, but the result is still self-contained.
    env RUSTFLAGS="-C target-feature=+crt-static" "$envname=$linker" \
        cargo build --release -p crabc-utils --target "$target"

    local out="$DIST/utils/$arch"
    mkdir -p "$out"
    cp "target/$target/release/crabc-utils" "$out/crabc-utils"
    echo "built $out/crabc-utils"
}

build x86_64 x86_64-unknown-linux-musl musl-gcc
build aarch64 aarch64-unknown-linux-musl aarch64-linux-gnu-gcc
build riscv64 riscv64gc-unknown-linux-musl riscv64-linux-gnu-gcc
