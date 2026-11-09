#!/usr/bin/env bash
# Check that every macro crabc and musl both define expands to the same token
# sequence on x86_64, aarch64 and riscv64.
#
# musl headers are generated from a pinned musl 1.2.6 tarball.  They are cached
# under $XDG_CACHE_HOME/crabc by default; set CRABC_MUSL_CACHE to override the
# cache location, or CRABC_MUSL_INC_ROOT to use pre-generated headers:
#
#   CRABC_MUSL_INC_ROOT=/path/to/musl-headers scripts/macro-parity.sh
#
# The directory layout for CRABC_MUSL_INC_ROOT is <arch>/include/bits/alltypes.h.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MUSL_VERSION=1.2.6
MUSL_URL="https://musl.libc.org/releases/musl-${MUSL_VERSION}.tar.gz"
MUSL_SHA256=d585fd3b613c66151fc3249e8ed44f77020cb5e6c1e635a616d3f9f82460512a
CACHE="${CRABC_MUSL_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/crabc}"

declare -A TARGET=(
    [x86_64]=x86_64-linux-musl
    [aarch64]=aarch64-linux-musl
    [riscv64]=riscv64-linux-musl
)
declare -A CC=(
    [x86_64]=gcc
    [aarch64]=aarch64-linux-gnu-gcc
    [riscv64]=riscv64-linux-gnu-gcc
)
declare -A CXX=(
    [x86_64]=g++
    [aarch64]=aarch64-linux-gnu-g++
    [riscv64]=riscv64-linux-gnu-g++
)

if [ -n "${CRABC_MUSL_INC_ROOT:-}" ]; then
    INC_ROOT="$CRABC_MUSL_INC_ROOT"
else
    INC_ROOT="$CACHE/musl-${MUSL_VERSION}-headers"
    if [ ! -f "$INC_ROOT/x86_64/include/bits/alltypes.h" ]; then
        mkdir -p "$CACHE"
        archive="$CACHE/musl-${MUSL_VERSION}.tar.gz"
        if [ ! -f "$archive" ]; then
            echo "downloading musl ${MUSL_VERSION} headers..."
            curl -fsSL --retry 3 "$MUSL_URL" -o "$archive"
        fi
        echo "${MUSL_SHA256}  ${archive}" | sha256sum -c -
        src="$CACHE/musl-${MUSL_VERSION}-src"
        rm -rf "$src"
        tar -xzf "$archive" -C "$CACHE"
        mv "$CACHE/musl-${MUSL_VERSION}" "$src"
        for arch in x86_64 aarch64 riscv64; do
            build="$CACHE/musl-${MUSL_VERSION}-build-${arch}"
            rm -rf "$build"
            cp -a "$src" "$build"
            (
                cd "$build"
                make distclean >/dev/null 2>&1 || true
                CC="${CC[$arch]}" ./configure --target="${TARGET[$arch]}" \
                    --prefix="$INC_ROOT/$arch" >/dev/null
                make install-headers >/dev/null
            )
        done
    fi
fi

for arch in x86_64 aarch64 riscv64; do
    for language in c c++; do
        if [ "$language" = c++ ]; then
            compiler="${CXX[$arch]}"
        else
            compiler="${CC[$arch]}"
        fi
        python3 "$ROOT/tests/macro_parity.py" \
            --arch "$arch" \
            --language "$language" \
            --musl "$INC_ROOT/$arch/include" \
            --crabc "$ROOT/include" \
            --cc "$compiler" \
            --allowlist "$ROOT/tests/macro-parity.allowlist"
    done
done
