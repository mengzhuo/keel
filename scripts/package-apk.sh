#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Build Alpine Linux .apk packages (apk-tools v3 / ADB format) from release
# artifacts.
#
# The package split mirrors Alpine's musl packages:
#   crabc         runtime libc + loader + crabc-loader
#   crabc-dev     headers, libc.a, CRT objects, linker stubs
#   crabc-utils   getconf/getent/iconv/ldd/ldconfig (Rust multicall binary)
#
# Usage: VERSION=0.7.6 scripts/package-apk.sh
#        CONTAINER_RUNTIME=podman scripts/package-apk.sh
#
# Expects the release artifacts produced by:
#   cargo build --release -p crabc-libc -p crabc-ldso --target x86_64-unknown-linux-gnu
#   CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
#     cargo build --release -p crabc --target x86_64-unknown-linux-musl
#   CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
#     cargo build --release -p crabc-libc -p crabc-ldso --target aarch64-unknown-linux-gnu
#   CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-gnu-gcc \
#     cargo build --release -p crabc --target aarch64-unknown-linux-musl
#   CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER=riscv64-linux-gnu-gcc \
#     cargo build --release --workspace --target riscv64gc-unknown-linux-musl
#   scripts/build-crt.sh
#   scripts/build-utils.sh
#
# Requires Docker or podman: the package is assembled with apk-tools 3
# (Alpine 3.23+), which writes the current v3/ADB format.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)}"
DIST="${DIST:-$ROOT/dist}"
REVISION="${REVISION:-r0}"
PKGVER="${VERSION}-${REVISION}"
IMAGE="${APK_BUILDER_IMAGE:-alpine:3.23}"

if [ -z "${CONTAINER_RUNTIME:-}" ]; then
    if command -v docker >/dev/null 2>&1; then
        CONTAINER_RUNTIME=docker
    elif command -v podman >/dev/null 2>&1; then
        CONTAINER_RUNTIME=podman
    else
        echo "error: docker or podman is required to run apk mkpkg" >&2
        exit 1
    fi
fi

mkdir -p "$DIST"

# mkpkg name arch build-root description depends provides replaces
mkpkg() {
    local name="$1" arch="$2" dir="$3" desc="$4"
    local depends="${5:-}" provides="${6:-}" replaces="${7:-}"

    "$CONTAINER_RUNTIME" run --rm \
        -e V="$PKGVER" -e ARCH="$arch" -e PKGNAME="$name" -e DESC="$desc" \
        -e DEPENDS="$depends" -e PROVIDES="$provides" -e REPLACES="$replaces" \
        -e SHA="${GITHUB_SHA:-}" \
        -v "$dir:/src:ro" -v "$DIST:/dist" "$IMAGE" sh -eu -c '
            rm -rf /pkg
            cp -a /src /pkg
            chown -R 0:0 /pkg
            chmod 755 /pkg
            set -- -F /pkg \
                -I "name:$PKGNAME" \
                -I "version:$V" \
                -I "description:$DESC" \
                -I "arch:$ARCH" \
                -I "license:BSD-3-Clause" \
                -I "origin:crabc" \
                -I "maintainer:Meng Zhuo <mengzhuo@iscas.ac.cn>" \
                -I "url:https://github.com/mengzhuo/crabc" \
                -I "build-time:$(date +%s)"
            [ -z "$DEPENDS" ] || set -- "$@" -I "depends:$DEPENDS"
            [ -z "$PROVIDES" ] || set -- "$@" -I "provides:$PROVIDES"
            [ -z "$REPLACES" ] || set -- "$@" -I "replaces:$REPLACES"
            if [ -n "$SHA" ]; then
                set -- "$@" -I "repo-commit:$SHA"
            fi
            set -- "$@" -o "/dist/$PKGNAME-$V.$ARCH.apk"
            apk mkpkg "$@"
        '
    echo "built $DIST/${name}-${PKGVER}.${arch}.apk"
}

package_runtime() {
    local arch="$1" libc="$2" ldso="$3" loader="$4"
    local dir
    dir="$(mktemp -d)"
    install -d "$dir/usr/lib/crabc" "$dir/usr/bin" "$dir/usr/share/doc/crabc" "$dir/lib"
    install -m 755 "$ROOT/$libc" "$dir/usr/lib/crabc/libc.so"
    install -m 755 "$ROOT/$ldso" "$dir/usr/lib/crabc/libldso.so"
    # The runtime package is a drop-in for Alpine's musl package.  These are
    # symlinks rather than copies so a single libc/loader serves both layouts.
    ln -s /usr/lib/crabc/libc.so "$dir/lib/libc.musl-${arch}.so.1"
    ln -s /usr/lib/crabc/libldso.so "$dir/lib/ld-musl-${arch}.so.1"
    # libgcc_s.so.1 (and other third-party DSOs) records the musl SONAME.
    # Keep a local alias so crabc-utils can resolve it through DT_RPATH
    # without installing crabc as the system libc.
    ln -s libc.so "$dir/usr/lib/crabc/libc.musl-${arch}.so.1"
    install -m 755 "$ROOT/$loader" "$dir/usr/bin/crabc-loader"
    install -m 644 "$ROOT/README.md" "$dir/usr/share/doc/crabc/README.md"
    mkpkg crabc "$arch" "$dir" \
        "A Rust no_std musl-compatible libc with a dynamic linker" \
        "" "musl=1.2.6-r0 so:libc.musl-${arch}.so.1=1.2.6-r0" "musl"
    rm -rf "$dir"
}

package_dev() {
    local arch="$1" libc_a="$2"
    local dir
    dir="$(mktemp -d)"
    install -d "$dir/usr/include" "$dir/usr/lib"

    cp -a "$ROOT/include/." "$dir/usr/include/"
    install -m 644 "$ROOT/$libc_a" "$dir/usr/lib/libc.a"
    install -m 644 "$DIST/crt/$arch/crt1.o" "$dir/usr/lib/crt1.o"
    install -m 644 "$DIST/crt/$arch/Scrt1.o" "$dir/usr/lib/Scrt1.o"
    install -m 644 "$DIST/crt/$arch/rcrt1.o" "$dir/usr/lib/rcrt1.o"
    install -m 644 "$DIST/crt/$arch/crti.o" "$dir/usr/lib/crti.o"
    install -m 644 "$DIST/crt/$arch/crtn.o" "$dir/usr/lib/crtn.o"

    # musl keeps the aliases in libc; empty archives let -lm, -lpthread and
    # friends appear in existing build systems without pulling in a second
    # implementation.
    for lib in libm libpthread librt libdl libcrypt libresolv libutil libxnet libssp_nonshared; do
        printf '!<arch>\n' > "$dir/usr/lib/$lib.a"
    done

    ln -s /usr/lib/crabc/libc.so "$dir/usr/lib/libc.so"
    mkpkg crabc-dev "$arch" "$dir" \
        "Development files for crabc (headers, libc.a and Rust-built CRT)" \
        "crabc=$PKGVER" "libc-dev=1.2.6-r0 musl-dev=1.2.6-r0" "musl-dev"
    rm -rf "$dir"
}

package_utils() {
    local arch="$1"
    local dir
    dir="$(mktemp -d)"
    install -d "$dir/usr/bin" "$dir/sbin"
    install -m 755 "$DIST/utils/$arch/crabc-utils" "$dir/usr/bin/crabc-utils"
    for cmd in getconf getent iconv ldd; do
        ln -s crabc-utils "$dir/usr/bin/$cmd"
    done
    ln -s ../usr/bin/crabc-utils "$dir/sbin/ldconfig"
    # This is the musl-utils replacement.  Alpine's solver treats
    # libc-utils/cmd:* as exclusive with musl-utils, so an existing
    # musl-utils must be removed first (the Dockerfile does this).
    mkpkg crabc-utils "$arch" "$dir" \
        "Rust implementations of musl-utils commands for crabc" \
        "crabc=$PKGVER" \
        "libc-utils=1.2.6-r0 musl-utils=1.2.6-r0 cmd:getconf=1.2.6-r0 cmd:getent=1.2.6-r0 cmd:iconv=1.2.6-r0 cmd:ldd=1.2.6-r0 cmd:ldconfig=1.2.6-r0" \
        "musl-utils"
    rm -rf "$dir"
}

# x86_64: the no_std libraries are built for the host (gnu) target, the
# loader (std) for musl so it runs on Alpine.
package_runtime x86_64 \
    target/x86_64-unknown-linux-gnu/release/libc.so \
    target/x86_64-unknown-linux-gnu/release/libldso.so \
    target/x86_64-unknown-linux-musl/release/loader
package_dev x86_64 target/x86_64-unknown-linux-gnu/release/libc.a
package_utils x86_64

# aarch64: the musl target forces crt-static and cannot build a cdylib with
# this toolchain, so build the no_std libraries for gnu and the loader for
# musl (the libraries do not depend on the C library).
package_runtime aarch64 \
    target/aarch64-unknown-linux-gnu/release/libc.so \
    target/aarch64-unknown-linux-gnu/release/libldso.so \
    target/aarch64-unknown-linux-musl/release/loader
package_dev aarch64 target/aarch64-unknown-linux-gnu/release/libc.a
package_utils aarch64

# riscv64: everything is built for the musl target.
package_runtime riscv64 \
    target/riscv64gc-unknown-linux-musl/release/libc.so \
    target/riscv64gc-unknown-linux-musl/release/libldso.so \
    target/riscv64gc-unknown-linux-musl/release/loader
package_dev riscv64 target/riscv64gc-unknown-linux-musl/release/libc.a
package_utils riscv64

(cd "$DIST" && sha256sum ./*.apk > SHA256SUMS)
echo "wrote $DIST/SHA256SUMS"
