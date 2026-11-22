#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Shared pinned-musl header setup, sourced by macro-parity.sh and
# inline-parity.sh.  Callers must enable `set -euo pipefail` before sourcing
# this file.
#
# The header tree is generated with musl's own mkalltypes.sed/syscall rules
# instead of running the full configure: the configure long-double probe
# rejects some valid cross compilers (notably powerpc64le), while the installed
# headers are compiler-independent.
#
# Sets:
#   MUSL_INC_ROOT  <MUSL_INC_ROOT>/<arch>/include holds the installed headers
#   MUSL_ARCHES    supported public architecture names
#   MUSL_CC        arch -> C compiler
#   MUSL_CXX       arch -> C++ compiler
#   MUSL_NM        arch -> binutils nm
#
# Override the header location with KEEL_MUSL_INC_ROOT (the layout must be
# <arch>/include/bits/alltypes.h), or the download cache with KEEL_MUSL_CACHE.

MUSL_VERSION=1.2.6
MUSL_URL="https://musl.libc.org/releases/musl-${MUSL_VERSION}.tar.gz"
MUSL_SHA256=d585fd3b613c66151fc3249e8ed44f77020cb5e6c1e635a616d3f9f82460512a
MUSL_CACHE="${KEEL_MUSL_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/keel}"

MUSL_ARCHES=(x86_64 aarch64 riscv64 powerpc64le loongarch64)
declare -A MUSL_ARCH_DIR=(
    [x86_64]=x86_64
    [aarch64]=aarch64
    [riscv64]=riscv64
    [powerpc64le]=powerpc64
    [loongarch64]=loongarch64
)
declare -A MUSL_CC=(
    [x86_64]=gcc
    [aarch64]=aarch64-linux-gnu-gcc
    [riscv64]=riscv64-linux-gnu-gcc
    [powerpc64le]=powerpc64le-linux-gnu-gcc
    [loongarch64]=loongarch64-linux-gnu-gcc
)
declare -A MUSL_CXX=(
    [x86_64]=g++
    [aarch64]=aarch64-linux-gnu-g++
    [riscv64]=riscv64-linux-gnu-g++
    [powerpc64le]=powerpc64le-linux-gnu-g++
    [loongarch64]=loongarch64-linux-gnu-g++
)
declare -A MUSL_NM=(
    [x86_64]=nm
    [aarch64]=aarch64-linux-gnu-nm
    [riscv64]=riscv64-linux-gnu-nm
    [powerpc64le]=powerpc64le-linux-gnu-nm
    [loongarch64]=loongarch64-linux-gnu-nm
)

# The repository pins GCC 14 for loongarch64; prefer it when both the
# versioned and the distribution-default compiler are installed.
if command -v loongarch64-linux-gnu-gcc-14 >/dev/null 2>&1; then
    MUSL_CC[loongarch64]=loongarch64-linux-gnu-gcc-14
fi
if command -v loongarch64-linux-gnu-g++-14 >/dev/null 2>&1; then
    MUSL_CXX[loongarch64]=loongarch64-linux-gnu-g++-14
fi

install_musl_headers() {
    local src="$1" arch="$2" dest="$3"
    local dir="${MUSL_ARCH_DIR[$arch]}"
    rm -rf "$dest"
    mkdir -p "$dest/include/bits"
    cp -R "$src/include/." "$dest/include/"
    cp -R "$src/arch/generic/bits/." "$dest/include/bits/"
    cp -R "$src/arch/$dir/bits/." "$dest/include/bits/"
    find "$dest/include" -name '*.in' -delete
    sed -f "$src/tools/mkalltypes.sed" \
        "$src/arch/$dir/bits/alltypes.h.in" "$src/include/alltypes.h.in" \
        > "$dest/include/bits/alltypes.h"
    cp "$src/arch/$dir/bits/syscall.h.in" "$dest/include/bits/syscall.h"
    sed -n -e 's/__NR_/SYS_/p' "$src/arch/$dir/bits/syscall.h.in" \
        >> "$dest/include/bits/syscall.h"
}

headers_ready() {
    local arch
    for arch in "${MUSL_ARCHES[@]}"; do
        [ -f "$MUSL_INC_ROOT/$arch/include/bits/alltypes.h" ] || return 1
    done
}

if [ -n "${KEEL_MUSL_INC_ROOT:-}" ]; then
    MUSL_INC_ROOT="$KEEL_MUSL_INC_ROOT"
else
    MUSL_INC_ROOT="$MUSL_CACHE/musl-${MUSL_VERSION}-headers"
    if ! headers_ready; then
        mkdir -p "$MUSL_CACHE"
        # Both precheck jobs can populate the cache.  Serialize the build so
        # parallel local runs cannot race on the same directory.
        if command -v flock >/dev/null 2>&1; then
            exec 9>"$MUSL_CACHE/.musl-headers.lock"
            flock 9
        fi
        if ! headers_ready; then
            archive="$MUSL_CACHE/musl-${MUSL_VERSION}.tar.gz"
            if [ ! -f "$archive" ]; then
                echo "downloading musl ${MUSL_VERSION} headers..." >&2
                curl -fsSL --retry 3 "$MUSL_URL" -o "$archive"
            fi
            echo "${MUSL_SHA256}  ${archive}" | sha256sum -c -
            src="$MUSL_CACHE/musl-${MUSL_VERSION}-src"
            rm -rf "$src"
            tar -xzf "$archive" -C "$MUSL_CACHE"
            mv "$MUSL_CACHE/musl-${MUSL_VERSION}" "$src"
            for arch in "${MUSL_ARCHES[@]}"; do
                install_musl_headers "$src" "$arch" "$MUSL_INC_ROOT/$arch"
            done
        fi
        if command -v flock >/dev/null 2>&1; then
            flock -u 9
            exec 9>&-
        fi
    fi
fi
