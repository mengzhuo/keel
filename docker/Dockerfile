# syntax=docker/dockerfile:1
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause


# A minimal Alpine image whose system libc and ELF interpreter are keel.
#
# Build (single arch):
#   docker build -f docker/Dockerfile -t keel:0.1.1 .
#
# Multi-arch:
#   docker buildx build --platform linux/amd64,linux/arm64,linux/riscv64 \
#     -f docker/Dockerfile -t keel:0.1.1 .
#
# The keel release APK is downloaded from GitHub. To use a local/mirrored
# package, pass KEEL_APK_URL with any URL reachable from the build container:
#   --build-arg KEEL_APK_URL=https://internal.example/keel-0.1.1-r0.x86_64.apk

ARG ALPINE_VERSION=3.23
ARG ALPINE_IMAGE=alpine
FROM ${ALPINE_IMAGE}:${ALPINE_VERSION}

ARG TARGETARCH
ARG KEEL_VERSION=0.1.1
ARG KEEL_APK_URL=
ARG KEEL_UTILS_URL=
# Optional build-time proxy settings (also accepted by BuildKit by name).
ARG http_proxy
ARG https_proxy
ARG no_proxy

RUN set -eux; \
    case "${TARGETARCH:-amd64}" in \
        amd64) arch=x86_64 ;; \
        arm64) arch=aarch64 ;; \
        riscv64) arch=riscv64 ;; \
        *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    apk add --no-cache ca-certificates curl; \
    url="${KEEL_APK_URL:-https://github.com/mengzhuo/keel/releases/download/v${KEEL_VERSION}/keel-${KEEL_VERSION}-r0.${arch}.apk}"; \
    utils_url="${KEEL_UTILS_URL:-https://github.com/mengzhuo/keel/releases/download/v${KEEL_VERSION}/keel-utils-${KEEL_VERSION}-r0.${arch}.apk}"; \
    curl -fsSL --retry 3 "$url" -o /tmp/keel.apk; \
    curl -fsSL --retry 3 "$utils_url" -o /tmp/keel-utils.apk; \
    apk add --allow-untrusted /tmp/keel.apk /tmp/keel-utils.apk; \
    rm -f /tmp/keel.apk /tmp/keel-utils.apk; \
    /bin/echo "keel ${KEEL_VERSION} (${arch})"; \
    /bin/sh -c 'echo "keel shell ok"'; \
    apk --version >/dev/null; \
    keel-utils getconf PAGESIZE >/dev/null; \
    ldd /bin/echo >/dev/null

# Some third-party DSOs record libc.so rather than the musl SONAME.
ENV LD_LIBRARY_PATH=/usr/lib/keel

CMD ["/bin/sh"]
