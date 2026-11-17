#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Cross-build and smoke-test keel for powerpc64le or loongarch64 under
# qemu-user.  The musl Rust targets cannot produce a cdylib, so libc.so is
# built for the matching -gnu target (the no_std code does not use glibc).
#
# Requirements:
#   rustup target add powerpc64le-unknown-linux-gnu loongarch64-unknown-linux-gnu
#   apt install gcc-powerpc64le-linux-gnu gcc-loongarch64-linux-gnu qemu-user
#
# Usage:
#   scripts/cross-smoke.sh                 # both architectures
#   scripts/cross-smoke.sh powerpc64le
#   scripts/cross-smoke.sh loongarch64
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ARCHES=("$@")
[ ${#ARCHES[@]} -eq 0 ] && ARCHES=(powerpc64le loongarch64)
FAILS=0

setup_arch() {
    case "$1" in
        powerpc64le)
            CC_BIN="${CROSS_CC:-powerpc64le-linux-gnu-gcc}"
            RUST_TARGET=powerpc64le-unknown-linux-gnu
            MUSL_TARGET=powerpc64le-unknown-linux-musl
            QEMU_BIN="${CROSS_QEMU:-qemu-ppc64le}"
            ENV_PREFIX=CARGO_TARGET_POWERPC64LE_UNKNOWN_LINUX_GNU_LINKER
            # Ubuntu 26.04's GCC defaults to IEEE binary128 long double.
            # keel and musl's ppc64le port follow the IBM double-double ABI.
            ABI_FLAGS=(-mabi=ibmlongdouble)
            ;;
        loongarch64)
            CC_BIN="${CROSS_CC:-loongarch64-linux-gnu-gcc}"
            RUST_TARGET=loongarch64-unknown-linux-gnu
            MUSL_TARGET=loongarch64-unknown-linux-musl
            QEMU_BIN="${CROSS_QEMU:-qemu-loongarch64}"
            ENV_PREFIX=CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_GNU_LINKER
            ABI_FLAGS=()
            ;;
        *)
            echo "unknown architecture: $1" >&2
            exit 2
            ;;
    esac
}

smoke_one() {
    local arch=$1
    setup_arch "$arch"
    if ! command -v "$CC_BIN" >/dev/null || ! command -v "$QEMU_BIN" >/dev/null; then
        echo "SKIP $arch: need $CC_BIN and $QEMU_BIN"
        return
    fi

    local TMP
    TMP="$(mktemp -d "/tmp/keel-$arch-smoke.XXXXXX")"
    trap 'rm -rf "$TMP"' RETURN
    local CARGO_TARGET="${CARGO_TARGET_DIR:-$ROOT/target}"
    local T="$CARGO_TARGET/$RUST_TARGET/debug"
    local LIBS="$TMP/libs"
    local CRT="$TMP/crt"
    local GCCINC
    GCCINC="$("$CC_BIN" -print-file-name=include)"
    mkdir -p "$LIBS" "$CRT"

    echo ">>> $arch: building libc.so and loader"
    (cd "$ROOT" && env "$ENV_PREFIX=$CC_BIN" CARGO_INCREMENTAL=0 \
        cargo build -q -p keel-libc --target "$RUST_TARGET") || {
        echo "BUILDFAIL $arch libc.so"; FAILS=$((FAILS + 1)); return; }
    (cd "$ROOT" && env "$ENV_PREFIX=$CC_BIN" CARGO_INCREMENTAL=0 \
        RUSTFLAGS='-C target-feature=+crt-static' \
        cargo build -q -p keel-rt --target "$RUST_TARGET") || {
        echo "BUILDFAIL $arch loader"; FAILS=$((FAILS + 1)); return; }

    # Startup objects are assembled by rustc for the musl target; no C
    # toolchain startup files are involved.
    local f
    for f in crt1 crti crtn rcrt1; do
        rustc --target "$MUSL_TARGET" --crate-type=lib --emit=obj -O \
            "$ROOT/crt/$f.rs" --out-dir "$CRT" >/dev/null 2>&1 || {
            echo "BUILDFAIL $arch crt/$f.rs"; FAILS=$((FAILS + 1)); return; }
    done
    cp "$CRT/crt1.o" "$CRT/Scrt1.o"

    local l
    for l in libc libpthread libm librt libcrypt libdl libresolv libutil; do
        ln -sf "$T/libc.so" "$LIBS/$l.so"
    done

    local CC=("$CC_BIN" "${ABI_FLAGS[@]}" -fPIE -pie -fno-builtin -nostdinc -isystem "$GCCINC"
        -I "$ROOT/include" -D_GNU_SOURCE -Wl,--no-as-needed)

    run() {
        local name=$1 src=$2 expected=${3:-}
        shift 3 2>/dev/null || shift $#
        local out="$TMP/$name"
        if ! (cd "$ROOT" && "${CC[@]}" -nostartfiles \
            -Wl,--dynamic-linker="$T/libc.so" -L"$LIBS" "$src" \
            "$CRT/crt1.o" "$@" -Wl,--allow-shlib-undefined -lc -o "$out") \
            >"$TMP/$name.build" 2>&1; then
            echo "BUILDFAIL $name: $(grep -m1 -E 'error:|undefined reference' "$TMP/$name.build" | head -c 160)"
            FAILS=$((FAILS + 1)); return
        fi
        LD_LIBRARY_PATH="$LIBS" timeout 30 "$QEMU_BIN" "$out" \
            >"$TMP/$name.out" 2>"$TMP/$name.err"
        local rc=$?
        if [ $rc -ne 0 ]; then
            echo "RUNFAIL $name rc=$rc stderr=$(head -c 160 "$TMP/$name.err" | tr '\n' ' ') stdout=$(head -c 160 "$TMP/$name.out" | tr '\n' ' ')"
            FAILS=$((FAILS + 1)); return
        fi
        if [ -n "$expected" ] && [ "$(cat "$TMP/$name.out")" != "$expected" ]; then
            echo "OUTFAIL $name: got '$(head -c 120 "$TMP/$name.out" | tr '\n' ' ')' want '$expected'"
            FAILS=$((FAILS + 1)); return
        fi
        echo "PASS $name: $(head -c 60 "$TMP/$name.out" | tr '\n' ' ')"
    }

    run hello "$ROOT/tests/fixtures/hello.c" hello
    run ctype "$ROOT/tests/fixtures/ctype_test.c"
    run string "$ROOT/tests/fixtures/string_test.c"
    run string_fast "$ROOT/tests/fixtures/string_fast_test.c"
    run stdlib "$ROOT/tests/fixtures/stdlib_test.c"
    run strtol "$ROOT/tests/fixtures/strtol_test.c"
    run strverscmp "$ROOT/tests/fixtures/strverscmp_test.c"
    run printf_bounds "$ROOT/tests/fixtures/printf_bounds_test.c"
    run stdio_full "$ROOT/tests/fixtures/stdio_full_test.c"
    run regex "$ROOT/tests/fixtures/regex_test.c"
    run fnmatch "$ROOT/tests/fixtures/fnmatch_test.c"
    run setjmp "$ROOT/tests/fixtures/setjmp_test.c" "setjmp ok"
    run fenv "$ROOT/tests/fixtures/fenv_test.c" OK
    run signal "$ROOT/tests/fixtures/signal_test.c"
    run process "$ROOT/tests/fixtures/process_test.c"
    run socket "$ROOT/tests/fixtures/socket_test.c"
    run net "$ROOT/tests/fixtures/net_test.c"
    run statvfs "$ROOT/tests/fixtures/statvfs_test.c"
    run pthread "$ROOT/tests/fixtures/pthread_test.c" "pthread ok"
    run pthread_full "$ROOT/tests/fixtures/pthread_full_test.c"
    run pthread_atfork "$ROOT/tests/fixtures/pthread_atfork_test.c"
    run lrand48 "$ROOT/tests/fixtures/lrand48_test.c"
    run locale "$ROOT/tests/fixtures/locale_test.c"
    run iconv "$ROOT/tests/fixtures/iconv_test.c"
    run time_regressions "$ROOT/tests/fixtures/time_regressions_test.c"
    run scandir "$ROOT/tests/fixtures/scandir_test.c"
    run math "$ROOT/tests/fixtures/math_test.c" "math ok"
    run va_list_abi "$ROOT/tests/fixtures/va_list_abi_test.c"
    run new_functions "$ROOT/tests/fixtures/new_functions_test.c" "new_functions ok"
    run compat "$ROOT/tests/fixtures/compat_test.c" "compat ok"
    run oflags "$ROOT/tests/fixtures/oflags_test.c" "oflags ok"
    run ucontext "$ROOT/tests/fixtures/ucontext_test.c" "ucontext ok"

    # Standalone loader: a static PIE with raw syscalls.
    if "$CC_BIN" "${ABI_FLAGS[@]}" -static-pie -nostdlib -fPIE \
        "$ROOT/tests/fixtures/tiny.c" -o "$TMP/tiny" >/dev/null 2>&1 \
        && [ "$(timeout 10 "$QEMU_BIN" "$T/keel" "$TMP/tiny" 2>/dev/null)" = Hello ]; then
        echo "PASS tiny-loader"
    else
        echo "FAIL tiny-loader"
        FAILS=$((FAILS + 1))
    fi

    # Per-thread TLS through pthread_create.
    run tls-threads "$ROOT/tests/fixtures/tlsthread_test.c" "tls threads ok"

    # dlopen a DSO that uses dynamic TLS.
    # Global-dynamic (ppc64le DTPMOD/DTPREL, loongarch64 TLSDESC) and
    # initial-exec (TPREL64) TLS in a dlopen'ed DSO.
    "$CC_BIN" "${ABI_FLAGS[@]}" -shared -nostdlib -fPIC -nostdinc -isystem "$GCCINC" \
        -I "$ROOT/include" -D_GNU_SOURCE \
        "$ROOT/tests/fixtures/dlopen_b.c" -o "$TMP/libb.so" >/dev/null 2>&1
    "$CC_BIN" "${ABI_FLAGS[@]}" -shared -nostdlib -fPIC -ftls-model=initial-exec \
        -nostdinc -isystem "$GCCINC" -I "$ROOT/include" -D_GNU_SOURCE \
        "$ROOT/tests/fixtures/dlopen_b.c" -o "$TMP/libb_ie.so" >/dev/null 2>&1
    if (cd "$ROOT" && "${CC[@]}" -nostartfiles -Wl,--dynamic-linker="$T/libc.so" \
        -L"$LIBS" "$ROOT/tests/fixtures/dlopen_tls_test.c" \
        "$CRT/crt1.o" -Wl,--allow-shlib-undefined -lc -o "$TMP/dlopen_tls") \
        >"$TMP/dlopen_tls.build" 2>&1; then
        local variant
        for variant in libb.so libb_ie.so; do
            if LD_LIBRARY_PATH="$LIBS" timeout 30 "$QEMU_BIN" "$TMP/dlopen_tls" \
                "$TMP/$variant" >"$TMP/dlopen_tls.out" 2>&1 \
                && grep -q "who=2 tls=7" "$TMP/dlopen_tls.out"; then
                echo "PASS dlopen-tls ($variant)"
            else
                echo "FAIL dlopen-tls ($variant): $(head -c 160 "$TMP/dlopen_tls.out" | tr '\n' ' ')"
                FAILS=$((FAILS + 1))
            fi
        done
    else
        echo "BUILDFAIL dlopen-tls"
        FAILS=$((FAILS + 1))
    fi

    rm -rf "$TMP"
}

for arch in "${ARCHES[@]}"; do
    smoke_one "$arch"
done
echo "cross smoke: FAILS=$FAILS"
exit "$FAILS"
