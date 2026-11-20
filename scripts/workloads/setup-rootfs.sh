#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Create the pristine Alpine rootfs used by the workload matrix.  The caller
# is expected to be root (CI uses `sudo -E`) and to have built the release
# libc first:
#
#   cargo build --release -p keel-libc --target x86_64-unknown-linux-gnu
#   sudo -E ./scripts/workloads/setup-rootfs.sh "$RUNNER_TEMP/workload"
#
# The result is <dest>/alpine-root and <dest>/libc.so.  run-workload.sh
# clones the rootfs and installs libc.so as the interpreter.

set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
DEST=${1:?usage: setup-rootfs.sh <dest>}

ALPINE_BRANCH=${ALPINE_BRANCH:-3.23}
ALPINE_VERSION=${ALPINE_VERSION:-3.23.8}
ALPINE_MIRROR=${ALPINE_MIRROR:-https://dl-cdn.alpinelinux.org}
MINIROOTFS="alpine-minirootfs-${ALPINE_VERSION}-x86_64.tar.gz"
MINIROOTFS_SHA256=${MINIROOTFS_SHA256:-56e2810205c17a5dff78cdd7b2c2af4ccdef16f18d765d5bb0f527438b136676}
TARGET=${TARGET:-x86_64-unknown-linux-gnu}
LIBC=${LIBC:-$ROOT/target/$TARGET/release/libc.so}

PACKAGES=(
    alpine-baselayout busybox bash curl git sqlite nginx redis python3 nodejs
    postgresql postgresql-client openssl ca-certificates
    php83 php83-openssl php83-sqlite3 php83-pdo_sqlite php83-mbstring
    php83-iconv php83-curl php83-phar php83-posix php83-pcntl php83-sockets
    ruby ffmpeg
)

ROOTFS=$DEST/alpine-root

[[ -f $LIBC ]] || {
    echo "error: $LIBC is missing; build keel-libc first" >&2
    exit 1
}

mkdir -p "$DEST"
rm -rf "$ROOTFS"
mkdir -p "$ROOTFS"

echo "==> downloading Alpine $ALPINE_VERSION minirootfs"
curl -fsSL --retry 3 --connect-timeout 20 --max-time 300 \
    -o "$DEST/$MINIROOTFS" \
    "$ALPINE_MIRROR/alpine/v$ALPINE_BRANCH/releases/x86_64/$MINIROOTFS"
echo "$MINIROOTFS_SHA256  $DEST/$MINIROOTFS" | sha256sum -c -
tar -xzf "$DEST/$MINIROOTFS" -C "$ROOTFS"
rm -f "$DEST/$MINIROOTFS"

# systemd-resolved's stub listener is reachable from the chroot; the
# non-stub file is more portable if it exists.
if [[ -f /run/systemd/resolve/resolv.conf ]]; then
    cp /run/systemd/resolve/resolv.conf "$ROOTFS/etc/resolv.conf"
else
    cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf"
fi

echo "==> installing workload packages"
for attempt in 1 2 3; do
    if timeout 900 chroot "$ROOTFS" /sbin/apk add --no-cache --no-progress \
        -X "$ALPINE_MIRROR/alpine/v$ALPINE_BRANCH/main" \
        -X "$ALPINE_MIRROR/alpine/v$ALPINE_BRANCH/community" \
        "${PACKAGES[@]}"; then
        break
    fi
    [[ $attempt -lt 3 ]] || exit 1
    echo "==> apk failed (attempt $attempt); retrying" >&2
    sleep 3
done

mkdir -p "$ROOTFS/work"
install -m 755 "$LIBC" "$DEST/libc.so"

echo "==> rootfs ready: $ROOTFS"
