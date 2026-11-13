# keel

keel is a Rust implementation of the musl C library ABI and dynamic linker
for Linux. It runs unmodified musl-linked binaries on x86\_64, aarch64 and
riscv64, and provides `libc.a` for static linking.

- musl 1.2.6 ABI with the full public symbol surface
- `libldso.so` as the ELF interpreter; no source changes or rebuilds
- `no_std` libc and loader, BSD-3-Clause

## Status

keel is experimental (`0.1.x`), not a production-ready drop-in replacement
for musl. It is validated with musl's `libc-test`, LTP, the Alpine workload
matrix, and CPython/Git/nginx upstream test suites on x86\_64, aarch64 and
riscv64.

## Build

```bash
cargo build --workspace
cargo test --workspace
```

Cross-compilation, packaging and release instructions are in `AGENTS.md` and
`docs/packages.md`.

## Documentation

- [Comparison with musl and other libcs](docs/comparison.md)
- [Real workload matrix](docs/workloads.md)
- [Upstream workload suites](docs/workload-suites.md)
- [LTP results](docs/ltp.md)
- [Performance](docs/performance.md)
- [Alpine packages](docs/packages.md)

keel is licensed under [BSD-3-Clause](LICENSE).
