#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Reproduce the GitHub Actions CI environment locally.
# Uses podman by default; falls back to docker if podman is unavailable.
# Requires a working container registry connection.
set -euo pipefail

RUNNER="${CI_RUNNER:-podman}"
if ! command -v "$RUNNER" >/dev/null 2>&1; then
    RUNNER=docker
fi

exec "$RUNNER" run --rm \
    -e HTTP_PROXY -e HTTPS_PROXY -e NO_PROXY \
    -v "$(pwd):/workspace" -w /workspace \
    "${CI_IMAGE:-ubuntu:24.04}" bash -c '
    export DEBIAN_FRONTEND=noninteractive
    apt-get -o Acquire::ForceIPv4=true update -qq
    apt-get -o Acquire::ForceIPv4=true install -y -qq curl musl-tools ca-certificates file gcc
    curl --proto '"'"'=https'"'"' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal
    . "$HOME/.cargo/env"
    touch /tmp/keel-ci-seed
    cargo build --workspace
    cargo test --workspace --no-fail-fast
'
