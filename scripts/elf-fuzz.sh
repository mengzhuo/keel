#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Mutation smoke-fuzz for the ELF loader: take a tiny valid DSO, flip a few
# bytes, and ask keel's dlopen() to load it in a subprocess.  A signal or
# abort is reported as a loader-robustness failure (the loader should reject
# malformed input with an error, not crash).  It is a smoke test, not
# coverage-guided; see docs/security-testing.md.
#
#   ./scripts/elf-fuzz.sh [iterations] [seed]

set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
ITERS=${1:-100}
SEED=${2:-1}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

if [[ ! -f target/debug/libc.so ]]; then
    cargo build -p keel-libc
fi

cat >"$TMP/lib.c" <<'EOF'
int fuzz_target(void) { return 1; }
EOF
musl-gcc -shared -nostartfiles -fPIC -Wl,-soname,libfuzz.so -o "$TMP/libfuzz.so" "$TMP/lib.c"

cat >"$TMP/dlopen.c" <<'EOF'
#include <dlfcn.h>
#include <stddef.h>
int main(int argc, char **argv) {
    if (argc < 2) return 2;
    void *h = dlopen(argv[1], RTLD_NOW);
    if (h) {
        /* Resolve but do not call: malformed code bytes are not the target. */
        (void)dlsym(h, "fuzz_target");
        dlclose(h);
    }
    return 0;
}
EOF
musl-gcc -O1 -fPIE -pie -I include -o "$TMP/dlopen" "$TMP/dlopen.c" \
    -Wl,--dynamic-linker="$ROOT/target/debug/libc.so" \
    -L target/debug -lc -Wl,--allow-shlib-undefined

python3 - "$TMP/libfuzz.so" "$TMP/mut.so" "$ITERS" "$SEED" "$TMP" <<'PY'
import os, random, subprocess, sys

src, dst, iters, seed, tmp = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]
data = open(src, "rb").read()
rng = random.Random(seed)
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
env = dict(os.environ)
env["LD_LIBRARY_PATH"] = os.path.join(root, "target", "debug")

# Do not mutate executable segments: executing corrupted code says nothing
# about the loader.  The mutation target is the ELF headers, dynamic table,
# relocations and data.
import struct
e_phoff = struct.unpack_from("<Q", data, 32)[0]
e_phentsize = struct.unpack_from("<H", data, 54)[0]
e_phnum = struct.unpack_from("<H", data, 56)[0]
exec_ranges = []
for p in range(e_phnum):
    off = e_phoff + p * e_phentsize
    p_type, p_flags = struct.unpack_from("<II", data, off)
    p_offset, = struct.unpack_from("<Q", data, off + 8)
    p_filesz, = struct.unpack_from("<Q", data, off + 32)
    if p_type == 1 and (p_flags & 1):
        exec_ranges.append((p_offset, p_offset + p_filesz))

def in_exec(pos):
    return any(lo <= pos < hi for lo, hi in exec_ranges)

def pick_pos():
    for _ in range(32):
        pos = rng.randrange(64, len(buf))
        if not in_exec(pos):
            return pos
    return rng.randrange(64, len(buf))

crashes = []
for i in range(iters):
    buf = bytearray(data)
    # 1-8 bit/byte flips, biased away from the first page so the ELF magic
    # still passes and the parser is actually reached.
    n = 1 + rng.randrange(8)
    for _ in range(n):
        pos = pick_pos()
        buf[pos] ^= 1 << rng.randrange(8)
    open(dst, "wb").write(buf)
    try:
        r = subprocess.run([os.path.join(tmp, "dlopen"), dst],
                           env=env, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=2)
        rc = r.returncode
    except subprocess.TimeoutExpired:
        rc = -1
    if rc < 0 or rc >= 128:
        crashes.append((i, rc))
        try:
            open("/tmp/opencode/elf-crash.so", "wb").write(buf)
        except OSError:
            pass
        if len(crashes) >= 5:
            break
if crashes:
    print("elf-fuzz: %d crash(es) in %d inputs: %s" % (len(crashes), i + 1, crashes), file=sys.stderr)
    sys.exit(1)
print("elf-fuzz: OK (%d mutated DSOs)" % iters)
PY
