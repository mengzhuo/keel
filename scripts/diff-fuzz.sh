#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Differential fuzz for the libc parsers that are easy to compare against a
# C library with the same POSIX surface.  The same deterministic corpus is
# fed to a keel-linked binary and a statically linked musl binary; any
# stdout/exit difference is reported.  This complements the hand-written
# parity tests:
#
#   ./scripts/diff-fuzz.sh [iterations]
#
# Requires musl-gcc and a built target/debug/libc.so.

set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
ITERS=${1:-2000}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if [[ ! -f target/debug/libc.so ]]; then
    cargo build -p keel-libc
fi

musl-gcc -O2 -fPIE -pie -I include -o "$TMP/keel" scripts/diff-fuzz.c \
    -Wl,--dynamic-linker="$ROOT/target/debug/libc.so" \
    -L target/debug -lc -Wl,--allow-shlib-undefined
# Use musl (statically linked by musl-gcc) as the oracle: glibc differs on
# deliberately strict inet_aton parsing and on trailing-backslash/
# unmatched-bracket fnmatch patterns, which are not keel regressions.
musl-gcc -O2 -static -o "$TMP/musl" scripts/diff-fuzz.c

LD_LIBRARY_PATH="$ROOT/target/debug" timeout 120 "$TMP/keel" "$ITERS" >"$TMP/keel.out"
timeout 120 "$TMP/musl" "$ITERS" >"$TMP/musl.out"

if diff -u "$TMP/musl.out" "$TMP/keel.out" >"$TMP/diff.txt"; then
    echo "diff-fuzz: OK ($ITERS inputs, musl oracle)"
else
    echo "diff-fuzz: differences found (musl -> keel):" >&2
    head -80 "$TMP/diff.txt" >&2
    exit 1
fi
