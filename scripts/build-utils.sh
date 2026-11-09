#!/usr/bin/env bash
# Build the crabc-utils multicall binary for each release architecture.
#
# The binary is a normal Rust program, but it is linked dynamically against
# crabc's libc and loader instead of musl:
#   interpreter  /usr/lib/crabc/libldso.so
#   NEEDED       libc.so (resolved via RUNPATH /usr/lib/crabc)
#
# libgcc_s.so.1 is required by Rust's std unwinder.  It is supplied at build
# time from Alpine's libgcc package; the APK declares the matching runtime
# dependency.
#
# Set LIBGCC_DIR_X86_64 / LIBGCC_DIR_AARCH64 / LIBGCC_DIR_RISCV64 to override
# the pre-fetched libgcc directories.  Without them the script extracts the
# package with Docker into dist/libgcc/<arch>.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="${DIST:-$ROOT/dist}"
IMAGE="${APK_BUILDER_IMAGE:-alpine:3.23}"
if [ -z "${CONTAINER_RUNTIME:-}" ]; then
    if command -v docker >/dev/null 2>&1; then
        CONTAINER_RUNTIME=docker
    elif command -v podman >/dev/null 2>&1; then
        CONTAINER_RUNTIME=podman
    else
        echo "error: docker or podman is required to fetch libgcc" >&2
        exit 1
    fi
fi

fetch_libgcc() {
    local arch="$1" apkarch="$2"
    local out="$DIST/libgcc/$arch"
    if [ -f "$out/libgcc_s.so.1" ]; then
        echo "$out"
        return
    fi
    if ! command -v "$CONTAINER_RUNTIME" >/dev/null 2>&1; then
        echo "error: $out/libgcc_s.so.1 is missing and $CONTAINER_RUNTIME is unavailable" >&2
        exit 1
    fi
    mkdir -p "$out"
    "$CONTAINER_RUNTIME" run --rm -v "$out:/out" "$IMAGE" sh -eu -c "
        cd /out
        apk fetch --arch '$apkarch' --allow-untrusted libgcc
        apk extract --allow-untrusted libgcc-*.apk
        mv usr/lib/libgcc_s.so.1 .
        rm -rf usr libgcc-*.apk
    " >&2
    echo "$out"
}

build() {
    local arch="$1" target="$2" linker="$3" libdir="$4" libgcc="$5"
    echo "building crabc-utils for $arch"
    local rustflags="-C target-feature=-crt-static \
        -C link-arg=-L$ROOT/$libdir \
        -C link-arg=-L$libgcc \
        -C link-arg=-Wl,--dynamic-linker=/usr/lib/crabc/libldso.so \
        -C link-arg=-Wl,--disable-new-dtags \
        -C link-arg=-Wl,-rpath,/usr/lib/crabc \
        -C link-arg=-Wl,--unresolved-symbols=ignore-all"
    local envname="CARGO_TARGET_$(echo "$target" | tr 'a-z-' 'A-Z_')_LINKER"
    env RUSTFLAGS="$rustflags" "$envname=$linker" \
        cargo build --release -p crabc-utils --target "$target"

    local out="$DIST/utils/$arch"
    mkdir -p "$out"
    cp "target/$target/release/crabc-utils" "$out/crabc-utils"
    echo "built $out/crabc-utils"
}

X86_LIBGCC="${LIBGCC_DIR_X86_64:-$(fetch_libgcc x86_64 x86_64)}"
ARM_LIBGCC="${LIBGCC_DIR_AARCH64:-$(fetch_libgcc aarch64 aarch64)}"
RISCV_LIBGCC="${LIBGCC_DIR_RISCV64:-$(fetch_libgcc riscv64 riscv64)}"

build x86_64 x86_64-unknown-linux-musl musl-gcc \
    target/x86_64-unknown-linux-gnu/release "$X86_LIBGCC"
build aarch64 aarch64-unknown-linux-musl aarch64-linux-gnu-gcc \
    target/aarch64-unknown-linux-gnu/release "$ARM_LIBGCC"
build riscv64 riscv64gc-unknown-linux-musl riscv64-linux-gnu-gcc \
    target/riscv64gc-unknown-linux-musl/release "$RISCV_LIBGCC"
