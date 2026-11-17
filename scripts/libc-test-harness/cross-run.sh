#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Cross-architecture libc-test runner under qemu-user.
#
# Usage: ./cross-run.sh <arch> [subset]
#   arch:   aarch64 | riscv64 | powerpc64le | loongarch64
#   subset: functional (default), math, regression, api, all
#
# The test driver (runtest.exe) is a static cross glibc binary; the test
# binaries are compiled against keel's headers and linked against the target
# libc.so.  Both are executed through qemu-user.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KEEL_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
ARCH="${1:?usage: cross-run.sh <arch> [subset]}"
SUBSET="${2:-functional}"
LIBC_TEST_DIR="${LIBC_TEST_DIR:-$(dirname "$KEEL_DIR")/libc-test}"
BUILD_ROOT="${BUILD_ROOT:-$SCRIPT_DIR/build}"
BUILD_DIR="$BUILD_ROOT/cross-$ARCH"
REPORT_DIR="$SCRIPT_DIR/reports"

case "$ARCH" in
    aarch64)
        CC="${CROSS_CC:-aarch64-linux-gnu-gcc}"
        QEMU="${CROSS_QEMU:-qemu-aarch64}"
        RUST_TARGET=aarch64-unknown-linux-gnu
        CRT_TARGET=aarch64-unknown-linux-gnu
        ABI_FLAGS=()
        ARCH_SKIP="functional/strtold functional/tls_local_exec"
        ;;
    riscv64)
        CC="${CROSS_CC:-riscv64-linux-gnu-gcc}"
        QEMU="${CROSS_QEMU:-qemu-riscv64}"
        RUST_TARGET=riscv64gc-unknown-linux-gnu
        CRT_TARGET=riscv64gc-unknown-linux-gnu
        ABI_FLAGS=()
        ARCH_SKIP="functional/strtold functional/tls_local_exec"
        ;;
    powerpc64le)
        CC="${CROSS_CC:-powerpc64le-linux-gnu-gcc}"
        QEMU="${CROSS_QEMU:-qemu-ppc64le}"
        RUST_TARGET=powerpc64le-unknown-linux-gnu
        CRT_TARGET=powerpc64le-unknown-linux-gnu
        # Ubuntu 26.04's GCC defaults to IEEE long double; keel/musl use IBM.
        ABI_FLAGS=(-mabi=ibmlongdouble)
        ARCH_SKIP="functional/strtold functional/tls_local_exec"
        ;;
    loongarch64)
        CC="${CROSS_CC:-loongarch64-linux-gnu-gcc}"
        QEMU="${CROSS_QEMU:-qemu-loongarch64}"
        RUST_TARGET=loongarch64-unknown-linux-gnu
        CRT_TARGET=loongarch64-unknown-linux-gnu
        ABI_FLAGS=()
        # QEMU 10.2 converts newfstatat through its 32-bit target_stat64
        # layout on loongarch64 (the 64-bit ABI struct is correct on real
        # hardware), so timestamp checks read garbage.  Re-enable this when
        # the CI qemu is fixed.
        ARCH_SKIP="functional/strtold functional/tls_local_exec functional/stat"
        ;;
    *)
        echo "unknown architecture: $ARCH" >&2
        exit 2
        ;;
esac

if ! command -v "$CC" >/dev/null || ! command -v "$QEMU" >/dev/null; then
    echo "SKIP $ARCH: need $CC and $QEMU" >&2
    exit 0
fi
if [ ! -d "$LIBC_TEST_DIR/src" ]; then
    echo "SKIP $ARCH: LIBC_TEST_DIR=$LIBC_TEST_DIR is not a libc-test checkout" >&2
    exit 0
fi

# qemu-user does not complete the robust-futex waiter handshake reliably;
# the native aarch64 CI still runs this test.
# qemu-user also does not model real-time signal queueing closely enough for
# raise-race's fork-in-handler counts, which flake between otherwise
# identical runs.  Keep it in the native suite.
SKIP_TESTS="math/acoshl math/asinhl math/lgammal math/tgammal regression/malloc-brk-fail functional/pthread_robust regression/pthread-robust-detach regression/raise-race"
if [ "$(id -u)" -eq 0 ]; then
    SKIP_TESTS="$SKIP_TESTS regression/pthread_atfork-errno-clobber"
fi
SKIP_TESTS="$SKIP_TESTS $ARCH_SKIP"
# tls_init_dlopen occasionally segfaults under qemu-ppc64le when the loader
# initializes a freshly dlopen'ed TLS module; the native ppc64le run and most
# qemu runs are clean, so retry it instead of dropping the coverage.
FLAKY_TESTS="${FLAKY_TESTS:-functional/ipc_shm functional/tls_init_dlopen regression/pthread_cond_wait-cancel_ignored}"

is_flaky() {
    case " $FLAKY_TESTS " in *" $1 "*) return 0 ;; esac
    return 1
}

GCCINC="$("$CC" -print-file-name=include)"
T="$KEEL_DIR/target/$RUST_TARGET/debug"
LIBC_SO="$T/libc.so"
FAKE_LIBS="$BUILD_DIR/fake-libs"
DRIVER_BUILD="$BUILD_DIR/driver"
TARGET_BUILD="$BUILD_DIR/target-common"
mkdir -p "$FAKE_LIBS" "$DRIVER_BUILD" "$TARGET_BUILD" "$REPORT_DIR"

echo ">>> $ARCH: building libc.so"
ENV_NAME="CARGO_TARGET_$(echo "$RUST_TARGET" | tr 'a-z-' 'A-Z_')_LINKER"
(cd "$KEEL_DIR" && env "$ENV_NAME=$CC" CARGO_INCREMENTAL=0 \
    cargo build -q -p keel-libc --target "$RUST_TARGET") || {
    echo "FATAL: libc.so build failed" >&2; exit 1; }
[ -f "$LIBC_SO" ] || { echo "FATAL: $LIBC_SO missing" >&2; exit 1; }

for lib in libc libpthread libm librt libcrypt libdl libresolv libutil; do
    ln -sf "$LIBC_SO" "$FAKE_LIBS/${lib}.so"
done

echo ">>> $ARCH: building crt1.o"
rustc --target "$CRT_TARGET" --crate-type=lib --emit=obj -O \
    "$KEEL_DIR/crt/crt1.rs" --out-dir "$BUILD_DIR" >/dev/null 2>&1 || {
    echo "FATAL: crt1.o build failed" >&2; exit 1; }

# Keep the driver and the tested library separate: the driver is a stock
# cross glibc program, while libtest-target.a follows keel's target ABI so
# that mtest.c's long double matches the test binaries.
BASE_CFLAGS="-I$LIBC_TEST_DIR/src/common -I$BUILD_DIR/common -pipe -std=c99 \
 -D_POSIX_C_SOURCE=200809L -Wall -Wno-unused-function -Wno-missing-braces \
 -Wno-unused -Wno-overflow -Wno-unknown-pragmas -fno-builtin -frounding-math \
 -Werror=implicit-function-declaration -Werror=implicit-int \
 -Werror=pointer-sign -Werror=pointer-arith -g -D_FILE_OFFSET_BITS=64"
TARGET_CFLAGS="$BASE_CFLAGS -nostdinc -isystem $GCCINC -I $KEEL_DIR/include ${ABI_FLAGS[*]:-}"

echo ">>> $ARCH: generating options.h and common objects"
mkdir -p "$BUILD_DIR/common"
if [ -f "$LIBC_TEST_DIR/src/common/options.h.in" ]; then
    "$CC" -E - < "$LIBC_TEST_DIR/src/common/options.h.in" | awk '
        /optiongroups_unistd_end/ {s=1; next}
        !s || !NF || /^#/ {next}
        !a {a=$1; if(NF==1)next}
        {print "#define "a" "$NF; a=""}' > "$BUILD_DIR/common/options.h" 2>/dev/null || true
fi

driver_objs=""
target_objs=""
for f in "$LIBC_TEST_DIR"/src/common/*.c; do
    base=$(basename "$f" .c)
    [ "$base" = "runtest" ] && continue
    "$CC" $BASE_CFLAGS -c -o "$DRIVER_BUILD/$base.o" "$f" \
        2>"$DRIVER_BUILD/$base.err" || true
    "$CC" $TARGET_CFLAGS -c -o "$TARGET_BUILD/$base.o" "$f" \
        2>"$TARGET_BUILD/$base.err" || true
    driver_objs="$driver_objs $DRIVER_BUILD/$base.o"
    target_objs="$target_objs $TARGET_BUILD/$base.o"
done

# The driver does not need mtest, but keeping a uniform archive is simpler.
ar rc "$DRIVER_BUILD/libtest.a" $driver_objs 2>/dev/null || true
ranlib "$DRIVER_BUILD/libtest.a" 2>/dev/null || true
"$CC" $BASE_CFLAGS -c -o "$DRIVER_BUILD/runtest.o" \
    "$LIBC_TEST_DIR/src/common/runtest.c" 2>"$DRIVER_BUILD/runtest.err" || true
"$CC" -static -g -o "$DRIVER_BUILD/runtest.exe" \
    "$DRIVER_BUILD/runtest.o" "$DRIVER_BUILD/libtest.a" \
    -lpthread -lm -lrt 2>"$DRIVER_BUILD/runtest.ld.err" || true
if [ ! -x "$DRIVER_BUILD/runtest.exe" ]; then
    echo "FATAL: failed to build the cross runtest.exe" >&2
    head -20 "$DRIVER_BUILD/runtest.err" "$DRIVER_BUILD/runtest.ld.err" >&2
    exit 1
fi

ar rc "$TARGET_BUILD/libtest.a" $target_objs 2>/dev/null || true
ranlib "$TARGET_BUILD/libtest.a" 2>/dev/null || true

case "$SUBSET" in
    all) DIRS="functional math regression api" ;;
    *)   DIRS="$SUBSET" ;;
esac

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RAW_REPORT="$REPORT_DIR/cross_${ARCH}_${SUBSET}_${TIMESTAMP}.txt"
SUMMARY_REPORT="$REPORT_DIR/cross_${ARCH}_${SUBSET}_${TIMESTAMP}.summary.txt"
> "$RAW_REPORT"

TOTAL=0 BUILDERROR=0 FAIL=0 PASS=0 SKIP=0 TIMEOUT=0
for dir in $DIRS; do
    SRC_DIR="$LIBC_TEST_DIR/src/$dir"
    if [ ! -d "$SRC_DIR" ]; then
        echo "WARNING: $SRC_DIR not found, skipping" >&2
        continue
    fi
    echo ">>> $ARCH: $dir"
    DIR_BUILD="$BUILD_DIR/$dir"
    mkdir -p "$DIR_BUILD"

    for dso_cfile in "$SRC_DIR"/*_dso.c; do
        [ -f "$dso_cfile" ] || continue
        dso_base=$(basename "$dso_cfile" .c)
        SO="$DIR_BUILD/${dso_base}.so"
        "$CC" $TARGET_CFLAGS -shared -fPIC -o "$SO" "$dso_cfile" \
            -L"$FAKE_LIBS" -lc 2>"$DIR_BUILD/${dso_base}.so.err" || true
        ln -sf "$SO" "$SRC_DIR/${dso_base}.so"
    done

    for cfile in "$SRC_DIR"/*.c; do
        [ -f "$cfile" ] || continue
        base=$(basename "$cfile" .c)
        [[ "$base" == *_dso ]] && continue
        TOTAL=$((TOTAL + 1))
        case " $SKIP_TESTS " in
            *" $dir/$base "*) echo "SKIP $dir/$base" >> "$RAW_REPORT"; SKIP=$((SKIP + 1)); continue ;;
        esac

        OBJ="$DIR_BUILD/${base}.o"
        if ! "$CC" $TARGET_CFLAGS -c -o "$OBJ" "$cfile" \
            2>"$DIR_BUILD/${base}.o.err"; then
            echo "BUILDERROR $dir/$base: compile failed" >> "$RAW_REPORT"
            head -5 "$DIR_BUILD/${base}.o.err" >> "$RAW_REPORT"
            BUILDERROR=$((BUILDERROR + 1)); continue
        fi
        if [ "$dir" = "api" ]; then
            echo "PASS $dir/$base" >> "$RAW_REPORT"; PASS=$((PASS + 1)); continue
        fi

        EXE="$DIR_BUILD/${base}.exe"
        COMPANION=()
        if [ -f "$DIR_BUILD/${base}_dso.so" ] && [ -f "$SRC_DIR/${base}.mk" ] && \
           grep -q 'LDLIBS.*_dso\.so' "$SRC_DIR/${base}.mk"; then
            COMPANION=("$DIR_BUILD/${base}_dso.so")
        fi
        if ! "$CC" -nostartfiles -rdynamic \
            -Wl,--dynamic-linker="$LIBC_SO" -Wl,--allow-shlib-undefined \
            -Wl,-rpath,'$ORIGIN' -o "$EXE" "$OBJ" "$BUILD_DIR/crt1.o" \
            "$TARGET_BUILD/libtest.a" "${COMPANION[@]}" \
            -L"$FAKE_LIBS" -Wl,--no-as-needed \
            -lpthread -lm -lrt -lcrypt -ldl -lresolv -lutil -lc \
            2>"$DIR_BUILD/${base}.ld.err"; then
            echo "BUILDERROR $dir/$base: link failed" >> "$RAW_REPORT"
            head -5 "$DIR_BUILD/${base}.ld.err" >> "$RAW_REPORT"
            BUILDERROR=$((BUILDERROR + 1)); continue
        fi

        ERRFILE="$DIR_BUILD/${base}.err"
        ATTEMPTS=0
        while :; do
            ATTEMPTS=$((ATTEMPTS + 1))
            RUN_RC=0
            (cd "$LIBC_TEST_DIR" && timeout 30 env LD_LIBRARY_PATH="$FAKE_LIBS" \
                "$QEMU" "$DRIVER_BUILD/runtest.exe" -w '' "$EXE" 2>"$ERRFILE") || RUN_RC=$?
            ACTUAL_ERRORS=""
            if [ $RUN_RC -eq 0 ]; then
                ACTUAL_ERRORS=$(grep -v "^ldso:\|^JUMP " "$ERRFILE" 2>/dev/null | grep -v "^$")
            fi
            if [ $RUN_RC -eq 0 ] && [ -z "$ACTUAL_ERRORS" ]; then
                echo "PASS $dir/$base" >> "$RAW_REPORT"; PASS=$((PASS + 1)); break
            fi
            if [ $ATTEMPTS -lt 5 ] && is_flaky "$dir/$base"; then continue; fi
            if [ $RUN_RC -eq 124 ]; then
                echo "TIMEOUT $dir/$base" >> "$RAW_REPORT"; TIMEOUT=$((TIMEOUT + 1))
            else
                echo "FAIL $dir/$base" >> "$RAW_REPORT"
                head -5 "$ERRFILE" >> "$RAW_REPORT" 2>/dev/null
                FAIL=$((FAIL + 1))
            fi
            break
        done
    done
done

cat > "$SUMMARY_REPORT" <<SUMMARY
libc-test Cross Report ($ARCH)
==============================
Date: $(date)
libc.so: $LIBC_SO
Subset: $SUBSET
Total: $TOTAL  PASS: $PASS  FAIL: $FAIL  BUILDERROR: $BUILDERROR  SKIP: $SKIP  TIMEOUT: $TIMEOUT
SUMMARY
ln -sf "$(basename "$SUMMARY_REPORT")" "$REPORT_DIR/cross_${ARCH}_latest-summary.txt"
ln -sf "$(basename "$RAW_REPORT")" "$REPORT_DIR/cross_${ARCH}_latest-raw.txt"
cat "$SUMMARY_REPORT"
[ $FAIL -eq 0 ] && [ $BUILDERROR -eq 0 ] && [ $TIMEOUT -eq 0 ]
