# crabc-libc

A `no_std` Rust implementation of musl-compatible libc. Produces `libc.so` and `libc.a` for running unmodified musl-linked ELF binaries.

## Usage

This crate is not intended for direct use as a Rust library. It produces C-compatible shared/static libraries:

- `libc.so` — dynamic library
- `libc.a` — static library

Build with:
```bash
cargo build -p crabc-libc
```

Output is in `target/debug/libc.so` and `target/debug/libc.a`.

## Features

- Implements ~350+ C library functions (stdio, stdlib, string, math, pthread, etc.)
- `no_std` — no Rust standard library dependency
- Targets the musl ABI for x86_64, aarch64 and riscv64
- Supports `f128` long double on aarch64 and riscv64
- Architecture selection and syscall numbers live in `src/arch.rs`

## License

MIT OR Apache-2.0
