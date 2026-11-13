#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Run upstream test suites for real programs against keel and a stock-musl
# baseline inside two identical Alpine chroots.
#
# Usage:
#   sudo ./scripts/workload-suites.sh [python] [git] [nginx]
#
# The default is all three suites.  Reports and downloaded sources live in
# workload-suites/ (ignored by git).
#
# Environment:
#   ALPINE_MIRROR   Alpine mirror root (default https://dl-cdn.alpinelinux.org)
#   ALPINE_VERSION  Alpine release (default 3.23)
#   SUITE_DIR       Working/report directory (default <repo>/workload-suites)
#   GIT_VERSION     Git source version for t/ (default 2.52.1)
#   NGINX_TESTS_REF nginx-tests commit (default pinned below)
#   STRICT=1        Fail even on documented known differences

set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
ALPINE_VERSION=${ALPINE_VERSION:-3.23}
ALPINE_MIRROR=${ALPINE_MIRROR:-https://dl-cdn.alpinelinux.org}
SUITE_DIR=${SUITE_DIR:-$ROOT/workload-suites}
WORK=$SUITE_DIR/work
REPORTS=$SUITE_DIR/reports
MUSL_ROOT=$SUITE_DIR/alpine-musl
KEEL_ROOT=$SUITE_DIR/alpine-keel
DOCKER=${DOCKER:-docker}
GIT_VERSION=${GIT_VERSION:-2.52.1}
NGINX_TESTS_REF=${NGINX_TESTS_REF:-bb4d348594a59e8e8d6e19f75943ac9e2df19a3a}
STRICT=${STRICT:-0}
TARGET=${TARGET:-x86_64-unknown-linux-gnu}
LOADER=ld-musl-x86_64.so.1
APK_REPOS=(-X "$ALPINE_MIRROR/alpine/v$ALPINE_VERSION/main"
           -X "$ALPINE_MIRROR/alpine/v$ALPINE_VERSION/community")

PACKAGES=(
    alpine-baselayout busybox python3 python3-tests
    git git-perl perl perl-utils perl-test-simple perl-io-socket-ssl perl-socket6
    nginx nginx-mod-http-geoip nginx-mod-http-image-filter nginx-mod-http-perl
    nginx-mod-http-xslt-filter nginx-mod-stream nginx-mod-stream-geoip nginx-mod-mail
    openssl ca-certificates curl tcl sqlite build-base zlib-dev
)

# Individually run so a failure in one test does not hide the others.
PYTHON_TESTS=(
    test_os test_io test_fcntl test_pwd test_grp test_socket test_select
    test_selectors test_threading test_subprocess test_signal test_ssl
    test_hashlib test_ctypes test_sqlite3 test_asyncio test_time test_math
    test_locale test_importlib test_multiprocessing test_queue test_logging
    test_datetime test_decimal test_re test_struct test_array test_binascii
    test_base64 test_json test_csv test_gzip test_tarfile test_zipfile
    test_shutil test_resource test_unicodedata test_codecs test_pickle
)

# All of these pass under the unmodified musl baseline.
GIT_TESTS=(
    t0000-basic.sh t0001-init.sh t0020-crlf.sh t0040-parse-options.sh
    t0060-path-utils.sh t3000-ls-files-others.sh t4000-diff-format.sh
)

# These eight do not need IPv6 and pass under the stock nginx package.
NGINX_TESTS=(
    autoindex.t rewrite.t headers.t gzip.t map.t ssi.t limit_req.t access_log.t
)

# Upstream/environment failures present in both runs are ignored.  test_ssl
# is the only known keel-only difference: its TLS 1.3 wrong-cert alert race
# can fail when the whole module runs, while the single test normally passes.
PYTHON_KNOWN_DIFFS=(
    test_ssl
)

MOUNTS=()
FAILED=0

log() { printf '==> %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

cleanup() {
    local i
    for ((i=${#MOUNTS[@]}-1; i>=0; i--)); do
        umount "${MOUNTS[i]}" 2>/dev/null || true
    done
}
trap cleanup EXIT

host_mount() {
    local src=$1 dst=$2
    mkdir -p "$dst"
    if mountpoint -q "$dst"; then
        return
    fi
    mount --bind "$src" "$dst"
    MOUNTS+=("$dst")
}

build_keel() {
    local libc=$ROOT/target/$TARGET/release/libc.so
    if [[ ! -f $libc ]]; then
        log "building keel release libc"
        (cd "$ROOT" && cargo build --release -p keel-libc --target "$TARGET")
    fi
    [[ -f $libc ]] || die "keel release libc is missing"
}

unmount_rootfs() {
    local rootfs=$1 m real
    real=$(realpath -m "$rootfs")
    while IFS= read -r m; do
        [[ -n $m ]] && umount -l "$m" 2>/dev/null || true
    done < <(grep -F " $real/" /proc/mounts | awk '{print $2}' | sort -r)
}

create_rootfs() {
    local rootfs=$1
    if [[ -f $rootfs/usr/bin/python3.12 ]]; then
        return
    fi
    log "creating $rootfs (Alpine $ALPINE_VERSION)"
    unmount_rootfs "$rootfs"
    rm -rf "$rootfs"
    mkdir -p "$rootfs"
    "$DOCKER" run --rm -v "$rootfs:/rootfs" "alpine:$ALPINE_VERSION" \
        apk --root /rootfs --initdb --arch x86_64 -U --allow-untrusted \
        "${APK_REPOS[@]}" add "${PACKAGES[@]}"
}

install_keel() {
    local libc=$ROOT/target/$TARGET/release/libc.so
    log "installing keel into $KEEL_ROOT"
    mkdir -p "$KEEL_ROOT/opt/keel"
    cp "$libc" "$KEEL_ROOT/opt/keel/libc.so"
    if [[ ! -f $KEEL_ROOT/lib/$LOADER.musl ]]; then
        cp "$KEEL_ROOT/lib/$LOADER" "$KEEL_ROOT/lib/$LOADER.musl"
    fi
    cp "$libc" "$KEEL_ROOT/lib/$LOADER"
    ln -sfn /opt/keel/libc.so "$KEEL_ROOT/lib/libc.musl-x86_64.so.1"
    # Building Git's test helper uses -lc; point the dev symlink at keel's
    # libc instead of the loader.
    ln -sfn /opt/keel/libc.so "$KEEL_ROOT/usr/lib/libc.so"
}

fetch_git() {
    local dir=$WORK/git
    if [[ -d $dir/t ]]; then
        return
    fi
    log "fetching Git $GIT_VERSION"
    rm -rf "$WORK"/git-* "$WORK/git"
    local tmp=$WORK/git.tar.gz
    curl -L --retry 2 -o "$tmp" \
        "https://codeload.github.com/git/git/tar.gz/refs/tags/v$GIT_VERSION"
    tar -xzf "$tmp" -C "$WORK"
    mv "$WORK/git-$GIT_VERSION" "$dir"
    rm -f "$tmp"
}

fetch_nginx_tests() {
    local dir=$WORK/nginx-tests
    if [[ -d $dir && -f $dir/lib/Test/Nginx.pm ]]; then
        return
    fi
    log "fetching nginx-tests $NGINX_TESTS_REF"
    rm -rf "$dir" "$WORK"/nginx-tests-*
    local tmp=$WORK/nginx-tests.tar.gz
    curl -L --retry 2 -o "$tmp" \
        "https://codeload.github.com/nginx/nginx-tests/tar.gz/$NGINX_TESTS_REF"
    tar -xzf "$tmp" -C "$WORK"
    mv "$WORK/nginx-tests-$NGINX_TESTS_REF" "$dir"
    rm -f "$tmp"
}

build_git_helper() {
    log "building Git t/helper/test-tool under keel"
    rm -f "$WORK/git/version-def.h" "$WORK/git/GIT-VERSION-FILE" \
        "$WORK/git/GIT-USER-AGENT" "$WORK/git/t/helper/test-tool"
    local flags="GIT_VERSION_OVERRIDE=$GIT_VERSION \
        NO_CURL=1 NO_EXPAT=1 NO_GETTEXT=1 NO_PERL=1 NO_PYTHON=1 \
        NO_OPENSSL=1 NO_REGEX=NeedsStartEnd"
    chroot "$KEEL_ROOT" /bin/sh -c "cd /work/git && \
        make $flags GIT-BUILD-OPTIONS && \
        make -j\"\$(nproc)\" $flags t/helper/test-tool"
    sed -i "s|^GIT_TEST_TEMPLATE_DIR=.*|GIT_TEST_TEMPLATE_DIR='/usr/share/git-core/templates'|" \
        "$WORK/git/GIT-BUILD-OPTIONS"
}

run_python() {
    local tag=$1 root=$2
    local out=$REPORTS/python-$tag.txt
    log "python suite ($tag)"
    chroot "$root" /usr/bin/python3 -m test -j 4 \
        -u all,-network,-audio,-gui "${PYTHON_TESTS[@]}" >"$out" 2>&1 || true
    awk '/tests? failed:/{flag=1;next} flag&&NF==0{flag=0} flag{print}' "$out" \
        | tr ' ' '\n' | sed '/^$/d' | sort -u \
        > "$REPORTS/python-$tag.failed"
}

run_git() {
    local tag=$1 root=$2
    local out=$REPORTS/git-$tag.txt failed=$REPORTS/git-$tag.failed
    : > "$out"; : > "$failed"
    log "git suite ($tag)"
    local t
    for t in "${GIT_TESTS[@]}"; do
        local logf=$REPORTS/git-$tag-$t.log
        if timeout 300 chroot "$root" /bin/sh -c \
            "cd /work/git/t && GIT_TEST_INSTALLED=/usr/bin \
             GIT_EXEC_PATH=/usr/libexec/git-core sh $t" >"$logf" 2>&1; then
            echo "PASS $t" >>"$out"
        else
            echo "FAIL $t" >>"$out"
            echo "$t" >>"$failed"
        fi
    done
}

run_nginx() {
    local tag=$1 root=$2
    local out=$REPORTS/nginx-$tag.txt failed=$REPORTS/nginx-$tag.failed
    : > "$out"; : > "$failed"
    log "nginx suite ($tag)"
    local t
    for t in "${NGINX_TESTS[@]}"; do
        local logf=$REPORTS/nginx-$tag-$t.log
        if timeout 180 chroot "$root" /bin/sh -c \
            "cd /work/nginx-tests && \
             TEST_NGINX_BINARY=/usr/sbin/nginx \
             TEST_NGINX_MODULES=/usr/lib/nginx/modules \
             TEST_NGINX_GLOBALS='user root;' prove -v $t" >"$logf" 2>&1; then
            echo "PASS $t" >>"$out"
        else
            echo "FAIL $t" >>"$out"
            echo "$t" >>"$failed"
        fi
    done
}

compare_sets() {
    local label=$1 musl=$2 keel=$3
    shift 3
    local known=("$@")
    local only mf cf
    mf=$(mktemp); cf=$(mktemp)
    sort -u "$musl" >"$mf"; sort -u "$keel" >"$cf"
    only=$(comm -13 "$mf" "$cf" || true)
    rm -f "$mf" "$cf"
    [[ -z $only ]] && return 0

    local unexpected=()
    local name
    while IFS= read -r name; do
        [[ -z $name ]] && continue
        if printf '%s\n' "${known[@]}" | grep -qx "$name"; then
            log "$label known difference: $name"
        else
            unexpected+=("$name")
        fi
    done <<<"$only"

    if ((${#unexpected[@]})); then
        printf 'keel-only %s failures:\n' "$label" >&2
        printf '  %s\n' "${unexpected[@]}" >&2
        FAILED=1
    fi
    if [[ $STRICT == 1 && -n $only ]]; then
        printf 'keel-only %s failures (strict):\n  %s\n' "$label" "$only" >&2
        FAILED=1
    fi
    return 0
}

summary() {
    local label=$1 musl=$2 keel=$3
    printf '\n%s\n' "$label"
    if [[ $label == python ]]; then
        grep -E 'Total tests:|tests? failed:|tests OK' "$musl" | tail -3 | sed 's/^/  musl:  /'
        grep -E 'Total tests:|tests? failed:|tests OK' "$keel" | tail -3 | sed 's/^/  keel: /'
    else
        printf '  musl:  %s passed, %s failed\n' \
            "$(grep -c '^PASS' "$musl" || true)" "$(grep -c '^FAIL' "$musl" || true)"
        printf '  keel: %s passed, %s failed\n' \
            "$(grep -c '^PASS' "$keel" || true)" "$(grep -c '^FAIL' "$keel" || true)"
    fi
    local only mf cf
    mf=$(mktemp); cf=$(mktemp)
    sort -u "$REPORTS/$label-musl.failed" >"$mf"
    sort -u "$REPORTS/$label-keel.failed" >"$cf"
    only=$(comm -13 "$mf" "$cf" || true)
    rm -f "$mf" "$cf"
    [[ -n $only ]] && printf '  keel-only: %s\n' "$(echo "$only" | tr '\n' ' ')"
    return 0
}

main() {
    command -v "$DOCKER" >/dev/null || die "$DOCKER is required"
    command -v chroot >/dev/null || die "chroot is required"
    [[ $(id -u) -eq 0 ]] || die "run as root (chroot and bind mounts)"
    mkdir -p "$WORK" "$REPORTS"

    local suites=("$@")
    ((${#suites[@]})) || suites=(python git nginx)

    build_keel
    create_rootfs "$MUSL_ROOT"
    if [[ ! -d $KEEL_ROOT/usr ]]; then
        log "cloning the baseline rootfs"
        cp -a "$MUSL_ROOT" "$KEEL_ROOT"
    fi
    install_keel

    host_mount /proc "$MUSL_ROOT/proc"
    host_mount /sys "$MUSL_ROOT/sys"
    host_mount /dev "$MUSL_ROOT/dev"
    host_mount /proc "$KEEL_ROOT/proc"
    host_mount /sys "$KEEL_ROOT/sys"
    host_mount /dev "$KEEL_ROOT/dev"
    host_mount "$WORK" "$MUSL_ROOT/work"
    host_mount "$WORK" "$KEEL_ROOT/work"

    fetch_git
    fetch_nginx_tests
    build_git_helper

    local s
    for s in "${suites[@]}"; do
        case $s in
        python) run_python musl "$MUSL_ROOT"; run_python keel "$KEEL_ROOT" ;;
        git) run_git musl "$MUSL_ROOT"; run_git keel "$KEEL_ROOT" ;;
        nginx) run_nginx musl "$MUSL_ROOT"; run_nginx keel "$KEEL_ROOT" ;;
        *) die "unknown suite: $s (use python, git or nginx)" ;;
        esac
    done

    for s in "${suites[@]}"; do
        case $s in
        python)
            compare_sets python "$REPORTS/python-musl.failed" \
                "$REPORTS/python-keel.failed" "${PYTHON_KNOWN_DIFFS[@]}"
            summary python "$REPORTS/python-musl.txt" "$REPORTS/python-keel.txt"
            ;;
        git)
            compare_sets git "$REPORTS/git-musl.failed" "$REPORTS/git-keel.failed"
            summary git "$REPORTS/git-musl.txt" "$REPORTS/git-keel.txt"
            ;;
        nginx)
            compare_sets nginx "$REPORTS/nginx-musl.failed" "$REPORTS/nginx-keel.failed"
            summary nginx "$REPORTS/nginx-musl.txt" "$REPORTS/nginx-keel.txt"
            ;;
        esac
    done

    if [[ $FAILED -ne 0 ]]; then
        printf '\nresult: FAIL\n' >&2
        exit 1
    fi
    printf '\nresult: PASS (known differences are documented in docs/workload-suites.md)\n'
}

main "$@"
