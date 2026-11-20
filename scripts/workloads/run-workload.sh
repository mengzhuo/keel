#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Run one workload in two identical Alpine chroots, one with stock musl and
# one with keel's libc.so, and compare the observable output.  The caller is
# expected to be root (CI uses `sudo -E`).
#
#   ./scripts/workloads/run-workload.sh <workload> <work-dir>
#
# <work-dir> must contain alpine-root/ and libc.so, as produced by
# setup-rootfs.sh.  The script creates alpine-keel/ and results/.

set -euo pipefail

NAME=${1:?usage: run-workload.sh <workload> <work-dir>}
WORK=${2:?usage: run-workload.sh <workload> <work-dir>}

# Localhost clients must not use a runner or lab proxy.  curl is the only
# workload that needs the external HTTPS check; leave its environment alone
# and make sure localhost still bypasses the proxy.
if [[ $NAME != curl ]]; then
    unset http_proxy https_proxy HTTP_PROXY HTTPS_PROXY
fi
export no_proxy="${no_proxy:+$no_proxy,}127.0.0.1,localhost"
export NO_PROXY="$no_proxy"

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$WORK/alpine-root
KEEL=$WORK/alpine-keel
LIBC=$WORK/libc.so
RESULTS=$WORK/results

[[ -d $ROOT ]] || { echo "error: $ROOT is missing" >&2; exit 1; }
[[ -f $LIBC ]] || { echo "error: $LIBC is missing" >&2; exit 1; }
[[ -x $HERE/$NAME.sh ]] || { echo "error: unknown workload $NAME" >&2; exit 1; }

mkdir -p "$RESULTS"
rm -f "$RESULTS/musl.out" "$RESULTS/keel.out" "$RESULTS/diff.txt" "$RESULTS/http.log"

MOUNTS=()
HTTP_PID=

cleanup() {
    if [[ -n ${HTTP_PID:-} ]]; then
        kill "$HTTP_PID" 2>/dev/null || true
        wait "$HTTP_PID" 2>/dev/null || true
    fi
    local i
    for ((i = ${#MOUNTS[@]} - 1; i >= 0; i--)); do
        umount "${MOUNTS[i]}" 2>/dev/null || true
    done
}
trap cleanup EXIT

# A previous failed run may have left mounts behind.
for root in "$ROOT" "$KEEL"; do
    for m in work proc dev sys; do
        umount "$root/$m" 2>/dev/null || true
    done
done

# Matrix jobs run on different VMs than the prepare job, so refresh the
# resolver configuration before cloning the rootfs.
if [[ -f /run/systemd/resolve/resolv.conf ]]; then
    cp /run/systemd/resolve/resolv.conf "$ROOT/etc/resolv.conf"
else
    cp /etc/resolv.conf "$ROOT/etc/resolv.conf"
fi

echo "==> installing keel into $KEEL"
rm -rf "$KEEL"
cp -a "$ROOT" "$KEEL"
mkdir -p "$KEEL/opt/keel"
cp "$LIBC" "$KEEL/opt/keel/libc.so"
cp "$KEEL/lib/ld-musl-x86_64.so.1" "$KEEL/lib/ld-musl-x86_64.so.1.musl"
cp "$LIBC" "$KEEL/lib/ld-musl-x86_64.so.1"
ln -sfn /opt/keel/libc.so "$KEEL/lib/libc.musl-x86_64.so.1"

echo "==> mounting /work, /proc, /dev and /sys"
for root in "$ROOT" "$KEEL"; do
    mkdir -p "$root/work" "$root/proc" "$root/dev" "$root/sys"
    mount --bind "$HERE" "$root/work"
    MOUNTS+=("$root/work")
    mount -t proc proc "$root/proc"
    MOUNTS+=("$root/proc")
    mount --bind /dev "$root/dev"
    MOUNTS+=("$root/dev")
    mount --bind /sys "$root/sys"
    MOUNTS+=("$root/sys")
done

# Local HTTP server for busybox wget, curl, node, php and ruby.
python3 -m http.server 18080 --bind 127.0.0.1 --directory "$HERE/web" \
    >"$RESULTS/http.log" 2>&1 &
HTTP_PID=$!
for _ in {1..25}; do
    if curl -fsS -o /dev/null http://127.0.0.1:18080/ 2>/dev/null; then
        break
    fi
    sleep 0.2
done
if ! curl -fsS -o /dev/null http://127.0.0.1:18080/ 2>/dev/null; then
    echo "error: local HTTP server did not start" >&2
    exit 1
fi

PG_UID=70
PG_GID=70
if [[ $NAME == postgres ]]; then
    PG_UID=$(chroot "$ROOT" /usr/bin/id -u postgres)
    PG_GID=$(chroot "$ROOT" /usr/bin/id -g postgres)
fi

run_one() {
    local root=$1 tag=$2 shell=/bin/sh
    [[ $NAME == bash ]] && shell=/bin/bash
    local -a userspec=()
    [[ $NAME == postgres ]] && userspec=(--userspec="$PG_UID:$PG_GID")

    echo "==> $NAME ($tag)"
    if ! { echo "### $tag"; timeout 300 chroot "${userspec[@]}" "$root" \
        "$shell" "/work/$NAME.sh"; } >"$RESULTS/$tag.out" 2>&1; then
        echo "error: $NAME failed under $tag" >&2
        cat "$RESULTS/$tag.out" >&2
        return 1
    fi
}

run_one "$ROOT" musl
run_one "$KEEL" keel

# Redis logs a PID and timestamp in its overcommit warning.  Everything else
# in the matrix is deterministic within a run.
normalize() {
    sed -E '/^### (musl|keel)$/d; /^[0-9]+:C .* # WARNING Memory overcommit/d' "$1"
}

if ! diff -u <(normalize "$RESULTS/musl.out") <(normalize "$RESULTS/keel.out") \
    >"$RESULTS/diff.txt"; then
    echo "::error::workload $NAME differs from the musl baseline"
    cat "$RESULTS/diff.txt" >&2
    echo "--- musl ---" >&2
    cat "$RESULTS/musl.out" >&2
    echo "--- keel ---" >&2
    cat "$RESULTS/keel.out" >&2
    exit 1
fi

grep -qx ".*_OK" "$RESULTS/keel.out" || {
    echo "error: $NAME did not print a completion marker" >&2
    exit 1
}

echo "$NAME: PASS"
