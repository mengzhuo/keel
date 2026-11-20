# keel

[![CI](https://github.com/mengzhuo/keel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/mengzhuo/keel/actions/workflows/ci.yml)
[![keel-libc](https://img.shields.io/crates/v/keel-libc.svg?label=keel-libc)](https://crates.io/crates/keel-libc)
[![docs.rs](https://img.shields.io/docsrs/keel-libc?label=docs.rs)](https://docs.rs/keel-libc)
[![downloads](https://img.shields.io/crates/d/keel-libc.svg?label=downloads)](https://crates.io/crates/keel-libc)
[![license](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)
[![rust](https://img.shields.io/badge/rust-stable-green.svg)](rust-toolchain.toml)
[![platforms](https://img.shields.io/badge/platforms-x86__64%20%7C%20aarch64%20%7C%20riscv64%20%7C%20ppc64le%20%7C%20loongarch64-blue.svg)](docs/comparison.md)

keel is a Rust implementation of the musl C library ABI and dynamic linker
for Linux. It runs unmodified musl-linked binaries on x86\_64, aarch64, riscv64,
powerpc64le and loongarch64, and provides `libc.a` for static linking.

- musl 1.2.6 ABI with the full public symbol surface
- `libc.so` as the ELF interpreter; no source changes or rebuilds
- `no_std` libc and dynamic linker (`libc.so`); the standalone `keel`
  mapper uses `std`

`keel` is the standalone mapper for static PIE and static ELF binaries;
dynamic binaries use `libc.so` as their interpreter.

## Status

keel is experimental (`0.1.x`), not a production-ready drop-in replacement
for musl. `libc-test` runs on x86\_64 and aarch64; LTP, the Alpine workload
matrix, and the CPython/Git/nginx upstream suites are recorded on x86\_64.
riscv64, powerpc64le and loongarch64 are covered by cross-builds and the
qemu smoke suites (`scripts/riscv-smoke.sh`, `scripts/cross-smoke.sh`).

## Workload validation

The [real workload matrix](docs/workloads.md) runs thirteen stock Alpine
programs in two identical chroots: one with the stock musl loader and one
with keel's `libc.so`. For each program the exit status and observable output
must match musl.

The current set is BusyBox, Bash, curl, Git, SQLite, nginx, Redis, CPython,
PHP, Ruby, Node.js, FFmpeg and PostgreSQL.  It exercises `dlopen`, TLS and
threads, `fork`/`exec`, sockets, `epoll`, SysV IPC, regular expressions,
`scandir`, termios/job control and the dynamic loader's dependency and
relocation paths.

The scripts live in `scripts/workloads/`.  CI runs one parallel job per
program after `build-and-test` through
[`.github/workflows/workloads.yml`](.github/workflows/workloads.yml), so a
failure in any program blocks `ci-status` and the release gate.

To run a workload locally:

```bash
cargo build --release -p keel-libc --target x86_64-unknown-linux-gnu
sudo ./scripts/workloads/setup-rootfs.sh /tmp/keel-workload
sudo ./scripts/workloads/run-workload.sh php /tmp/keel-workload
```

`setup-rootfs.sh` pins Alpine 3.23.8 and verifies the minirootfs checksum.
Set `ALPINE_MIRROR` to use a mirror, and `LIBC` if the release `libc.so` is in
a different target directory.

## Build

The toolchain is pinned to stable Rust by `rust-toolchain.toml`; Rust 1.99 or newer is required for stable C varargs.  The
integration tests need `musl-gcc`, `file` and binutils; the static fixtures
also need the host musl CRT objects under `/usr/lib/<arch>-linux-musl/`.

```bash
cargo build --workspace
cargo test --workspace
```

`cargo test --workspace` expects `target/debug/libc.so` from the build above.
Cross-compilation and packaging are in [`docs/packages.md`](docs/packages.md);
release workflow details are in `AGENTS.md`.

## Documentation

- [Comparison with musl and other libcs](docs/comparison.md)
- [Known limitations](docs/limitations.md)
- [Real workload matrix](docs/workloads.md)
- [Upstream workload suites](docs/workload-suites.md)
- [LTP results](docs/ltp.md)
- [Performance](docs/performance.md)
- [Alpine packages](docs/packages.md)

keel is licensed under [BSD-3-Clause](LICENSE). Parts ported from musl
retain its MIT notice, reproduced in [NOTICE](NOTICE).
