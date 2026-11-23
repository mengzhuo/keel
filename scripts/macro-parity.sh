#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Check that every macro keel and musl both define expands to the same token
# sequence on x86_64, aarch64 and riscv64.
#
# musl headers are generated from a pinned musl 1.2.6 tarball.  They are cached
# under $XDG_CACHE_HOME/keel by default; set KEEL_MUSL_CACHE to override the
# cache location, or KEEL_MUSL_INC_ROOT to use pre-generated headers:
#
#   KEEL_MUSL_INC_ROOT=/path/to/musl-headers scripts/macro-parity.sh
#
# The directory layout for KEEL_MUSL_INC_ROOT is <arch>/include/bits/alltypes.h.
#
# The arch/language combinations are independent, so run them in parallel and
# print their logs in a stable order.  This keeps the check's wall time close
# to one combination instead of the sum of all six.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/musl-headers.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

logs=()
status=0

for arch in x86_64 aarch64 riscv64; do
    for language in c c++; do
        if [ "$language" = c++ ]; then
            compiler="${MUSL_CXX[$arch]}"
        else
            compiler="${MUSL_CC[$arch]}"
        fi
        log="$tmp/$arch-$language.log"
        logs+=("$log")
        (
            if python3 "$ROOT/tests/macro_parity.py" \
                --arch "$arch" \
                --language "$language" \
                --musl "$MUSL_INC_ROOT/$arch/include" \
                --keel "$ROOT/include" \
                --cc "$compiler" \
                --allowlist "$ROOT/tests/macro-parity.allowlist" \
                >"$log" 2>&1; then
                rc=0
            else
                rc=$?
            fi
            printf '%s\n' "$rc" > "$log.rc"
            exit 0
        ) &
    done
done

wait

for log in "${logs[@]}"; do
    cat "$log"
    rc="$(cat "$log.rc" 2>/dev/null || echo 1)"
    if [ "$rc" -ne 0 ]; then
        status=1
    fi
done

exit "$status"
