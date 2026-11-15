# keel

keel is a Rust implementation of the musl C library ABI and dynamic linker
for Linux. It runs unmodified musl-linked binaries on x86\_64, aarch64 and
riscv64, and provides `libc.a` for static linking.

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
riscv64 is covered by the cross-build and the qemu smoke suite.

## Build

The toolchain is pinned to Rust nightly by `rust-toolchain.toml`.  The
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

keel is licensed under [BSD-3-Clause](LICENSE).
