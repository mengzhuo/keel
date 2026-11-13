#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Run cargo build/test --workspace in the latest Alpine container.
# Requires podman or docker, and network access for apk/rustup.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUNNER="${CI_RUNNER:-podman}"
if ! command -v "$RUNNER" >/dev/null 2>&1; then
    RUNNER=docker
fi

exec "$RUNNER" run --rm \
    -e HTTP_PROXY -e HTTPS_PROXY -e NO_PROXY \
    -v "$ROOT:/workspace" -w /workspace \
    "${CI_IMAGE:-alpine:latest}" sh -c '
    set -e
    apk add --no-cache bash file binutils build-base rustup >/dev/null
    rustup-init -y --profile minimal --default-toolchain nightly
    . "$HOME/.cargo/env"

    # The test harness expects Debian-style musl-gcc and CRT paths.
    printf "%s\n" "#!/bin/sh" "for a in \"\$@\"; do [ \"\$a\" = \"-static\" ] && exec /usr/bin/gcc -no-pie \"\$@\"; done" "exec /usr/bin/gcc \"\$@\"" > /usr/local/bin/musl-gcc
    chmod +x /usr/local/bin/musl-gcc
    mkdir -p /usr/lib/x86_64-linux-musl
    for f in crt1.o Scrt1.o crti.o crtn.o; do
        ln -sf "/usr/lib/$f" "/usr/lib/x86_64-linux-musl/$f"
    done

    # compat checks glob("/tmp/*"), which is empty in a fresh container.
    touch /tmp/keel-ci-seed

    # musl targets default to crt-static, which drops the cdylib.
    export RUSTFLAGS="-C target-feature=-crt-static"
    cargo clean
    cargo build --workspace
    cargo test --workspace --no-fail-fast
'
