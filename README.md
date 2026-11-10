# crabc libc

crabc is a BSD-3-Clause licensed implementation of the musl C library ABI
and dynamic linker for Linux, written in Rust. It runs unmodified musl-linked
binaries on x86\_64, aarch64 and riscv64 when `libldso.so` is used as the
interpreter, and provides `libc.a` for static linking. crabc is built on the
principle that a libc should be small, easy to audit and safe to maintain;
`unsafe` is confined to syscalls and pointer glue.

crabc tracks musl 1.2.6 and exports its full public symbol surface. It is
validated with musl's own libc-test and a differential harness that runs the
same binaries under musl and crabc. CI also checks that every musl ABI macro
is present on x86\_64, aarch64 and riscv64, expands to the same tokens as musl
in both C and C++, and that numeric header macros agree with the corresponding
Rust constants. It is faster than musl on both x86\_64 and riscv64 (qemu-user)
in the microbenchmark suite; process startup is the one structural exception.
See `docs/performance.md`.

Real, unmodified Alpine binaries are tracked in the
[workload matrix](docs/workloads.md): BusyBox, curl, Git, SQLite, nginx,
Redis, CPython, Node.js and PostgreSQL all run under crabc, with a per-project
report and baseline comparison. The [LTP report](docs/ltp.md) records the
`syscalls`, math, memory-management, scheduler, threading and SysV IPC suites
against the same binaries under musl and crabc; the shared suites have
identical failure lists. Release APKs are split the same way as Alpine's
musl packages: `crabc` (runtime), `crabc-dev` (headers, `libc.a`, Rust-built
CRT objects) and `crabc-utils` (`getconf`, `getent`, `iconv`, `ldd`,
`ldconfig`). See [`docs/packages.md`](docs/packages.md). A Docker image that
uses crabc as the system libc is built from
[`docker/Dockerfile`](docker/README.md).

crabc is not the first C library written in Rust, and it is not a musl fork:
it is an independent implementation tracking the musl ABI. See
[docs/comparison.md](docs/comparison.md) for how it relates to musl, relibc,
eyra, and other projects.

## Status and scope

crabc is experimental (`0.7.x`). It is not a production-ready drop-in
replacement for musl. The validated scope is the workload matrix and LTP
report above; functionality outside that scope is not claimed.

An earlier README overstated Rust's safety guarantees, implying that the
implementation was free of buffer overflows, use-after-free, or undefined
behavior. That was inaccurate. Like every libc, crabc contains `unsafe` code
at the syscall and pointer boundary. Rust reduces the unsafe surface; it does
not eliminate it. Claims in this repository are limited to what the test
suites and workload matrix demonstrate.

Build and cross-compilation instructions are in `AGENTS.md`. Release builds
are size-optimized and stripped, and are produced from `v*` tags; the
workspace is published as the `crabc`, `crabc-libc` and `crabc-ldso` crates.

Documentation: [comparison](docs/comparison.md), [workloads](docs/workloads.md),
[LTP](docs/ltp.md), [performance](docs/performance.md).

crabc is licensed under [BSD-3-Clause](LICENSE).
