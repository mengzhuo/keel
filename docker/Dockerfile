# syntax=docker/dockerfile:1
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause


# A minimal Alpine image whose system libc and ELF interpreter are crabc.
#
# Build (single arch):
#   docker build -f docker/Dockerfile -t crabc:0.7.6 .
#
# Multi-arch:
#   docker buildx build --platform linux/amd64,linux/arm64,linux/riscv64 \
#     -f docker/Dockerfile -t crabc:0.7.6 .
#
# The crabc release APK is downloaded from GitHub. To use a local/mirrored
# package, pass CRABC_APK_URL with any URL reachable from the build container:
#   --build-arg CRABC_APK_URL=https://internal.example/crabc-0.7.6-r0.x86_64.apk

ARG ALPINE_VERSION=3.23
ARG ALPINE_IMAGE=alpine
FROM ${ALPINE_IMAGE}:${ALPINE_VERSION}

ARG TARGETARCH
ARG CRABC_VERSION=0.7.6
ARG CRABC_APK_URL=
ARG CRABC_UTILS_URL=
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
    url="${CRABC_APK_URL:-https://github.com/mengzhuo/crabc/releases/download/v${CRABC_VERSION}/crabc-${CRABC_VERSION}-r0.${arch}.apk}"; \
    utils_url="${CRABC_UTILS_URL:-https://github.com/mengzhuo/crabc/releases/download/v${CRABC_VERSION}/crabc-utils-${CRABC_VERSION}-r0.${arch}.apk}"; \
    curl -fsSL --retry 3 "$url" -o /tmp/crabc.apk; \
    curl -fsSL --retry 3 "$utils_url" -o /tmp/crabc-utils.apk; \
    apk add --allow-untrusted /tmp/crabc.apk /tmp/crabc-utils.apk; \
    rm -f /tmp/crabc.apk /tmp/crabc-utils.apk; \
    /bin/echo "crabc ${CRABC_VERSION} (${arch})"; \
    /bin/sh -c 'echo "crabc shell ok"'; \
    apk --version >/dev/null; \
    crabc-utils getconf PAGESIZE >/dev/null; \
    ldd /bin/echo >/dev/null

# Some third-party DSOs record libc.so rather than the musl SONAME.
ENV LD_LIBRARY_PATH=/usr/lib/crabc

CMD ["/bin/sh"]
