# Related projects

keel is an independent implementation of the musl C library ABI and dynamic
linker. This page records how it relates to other libc and dynamic-linker
projects. It is a snapshot, not a scorecard: activity and scope change, and a
project being listed here does not imply that it is complete or production
ready.

Last checked: 2026-11-09.

## What keel is

- An independent implementation of the musl 1.2.6 ABI, not a fork of musl.
- `libc.so`, `libc.a`, and `libldso.so` for x86\_64, aarch64, and riscv64.
- `no_std`; `unsafe` is confined to the syscall and pointer boundary.
- Validated with musl's own libc-test, LTP, a differential harness against
  musl 1.2.6, and nine real Alpine workloads.
- Experimental (`0.8.x`), not a production-ready drop-in replacement.

## What keel is not

- It is not the first C library written in Rust. relibc, eyra, and anp/rusl
  came earlier.
- It does not target the glibc ABI; binaries built against glibc are not
  expected to run.
- It does not claim to replace musl in production today.
- It does not claim that Rust makes the whole implementation memory-safe.
  The implementation still contains `unsafe` code at the syscall and pointer
  boundary.

## Directly related: musl ABI in Rust

| Project | Status | Scope | Notes |
|---|---|---|---|
| **keel** | active, `0.8.x` | musl 1.2.6; x86\_64, aarch64, riscv64; `libldso.so` | 9/9 Alpine workloads, shared LTP suites have identical failure lists, libc-test 414 PASS + 5 SKIP, full public macro surface checked |
| [anp/rusl](https://github.com/anp/rusl) | inactive (2018) | x86\_64; partial musl port | historical, proof-of-concept port |
| [SrDicov/zlibc](https://github.com/SrDicov/zlibc) | pre-alpha (2026-10) | x86\_64; musl ABI | phase 1 skeleton; does not link or run yet; no ldso |
| [starkross/mytilus](https://github.com/starkross/mytilus) | pre-alpha (2026-05) | aarch64 only; musl 1.2.6 | empty crate scaffolds; no working libc; no ldso |
| [flying-rind/rusl](https://github.com/flying-rind/rusl) | experiment (2026-06) | musl | no README or validation; experimental |

There are also several hobby `zlibc` projects that target hobby operating
systems rather than the Linux musl ABI; they are not listed here.

## Other Rust libc implementations

| Project | Status | ABI / target | Dynamic linking | Notes |
|---|---|---|---|---|
| [relibc](https://github.com/redox-os/relibc) | active | Redox and Linux; POSIX, not the musl ABI | has an `ld_so` (Redox-oriented) | the closest "Rust libc + loader" project |
| [eyra](https://github.com/sunfishcode/eyra) / [c-ward](https://github.com/sunfishcode/c-ward) / [mustang](https://github.com/sunfishcode/mustang) | archived | glibc ABI | no; static-only | formerly the most visible Rust libc effort |
| [Liella](https://github.com/duskglow/liella) | active, WIP | glibc 2.39; x86-64 | in progress | static PIE works; shared objects are not loaded yet |
| [ouma](https://github.com/rseclinux/ouma) | active | hardened GNU/Linux; own ABI | no standalone ldso documented | AGPL-3.0; hardening-oriented |
| [FrankenLibC](https://github.com/Dicklesworthstone/frankenlibc) | active | glibc-shaped `libc.so` | no standalone ldso | LD_PRELOAD interposer; classifies its exported symbols |
| [rulibc](https://github.com/rulibc/rulibc) | inactive (2020) | relibc fork; Windows/Linux | not documented | Windows/Linux focus |
| [rlibc](https://github.com/mahkoh/rlibc) | inactive (2015) | partial C99/POSIX | no | historical |
| [tinyrlibc](https://github.com/rust-embedded-community/tinyrlibc) | active | bare-metal | no | not a Linux libc |
| [slibc](https://slopos.sloplabs.net/docs/architecture/userland) (SlopOS) | part of SlopOS | SlopOS POSIX | has an `ld_so` | not a Linux drop-in |

## Rust dynamic linkers

| Project | Status | Scope | Notes |
|---|---|---|---|
| [rust-dynlinker / dlopen-rs](https://github.com/weizhiao/rust-dynlinker) | active | ELF loader, `dlopen`/`dlsym`, `RTLD_NEXT`, TLS, RELR | experimental rtld on x86\_64; not a libc |
| [Miros](https://github.com/5-pebbles/miros) | active | ldso + partial libc + pthreads | x86\_64; not musl ABI |
| [rustld](https://github.com/louislafosse/rustld) | active | static and dynamic ELF loader | x86\_64/aarch64; glibc and musl binaries; not a full libc |
| [dryad](https://github.com/m4b/dryad) | inactive (2018) | ELF dynamic linker prototype | x86\_64; historical |
| [elf_loader / Relink](https://github.com/weizhiao/Relink) | active | `no_std` ELF loading and runtime linking library | building block, not a libc |
| [libdlink](https://github.com/hehaoqian/libdlink) | WIP | dynamic linker experiment | early stage |

These projects solve overlapping problems and are useful references for ELF
loading, TLS, `RTLD_NEXT`, and relocation handling. They are not substitutes
for a libc.

## Established C libraries

| Library | Language | Target | Notes |
|---|---|---|---|
| **musl** | C | Linux musl ABI | the compatibility target for keel; mature and production-ready |
| **glibc** | C | Linux GNU ABI | full-featured; different ABI from musl |
| **LLVM libc** | C++ | Linux, bare-metal, GPU | industry-backed; no standalone dynamic linker |
| **mlibc** | C++ | portable POSIX | used by Managarm |
| **Cosmopolitan** | C | portable / APE | different portability goal; APE loader |
| **bionic** | C/assembly | Android | Android-only |
| **uClibc-ng / newlib / picolibc** | C | embedded | not musl ABI |
| **wasi-libc** | C | WASI | WebAssembly target |

## Common questions

### Why not musl?

musl is excellent, mature, and production-ready; keel is not a replacement
today. keel explores a different point in the design space: the musl ABI,
implemented in Rust, with an integrated dynamic linker and a `no_std` core.
Use musl for production; use keel to experiment, audit, or work on libc
internals.

### Why not relibc?

relibc is Redox-first and has its own ABI. keel targets the musl ABI and
stock Alpine binaries. The projects share the "libc in Rust" direction but
have different goals.

### Why not eyra?

eyra implements the glibc ABI, is static-only, and its repositories are
archived. keel implements the musl ABI, supports dynamic linking, and is
actively maintained. They are not drop-in replacements for each other.

### Why not zlibc or mytilus?

Both are pre-alpha: they do not have a working libc or dynamic linker yet and
are limited to one architecture. keel has a working `libldso.so`, three
architectures, nine real Alpine workloads, and identical LTP failure lists on
the shared suites. More Rust libc projects are good for the ecosystem; these
are simply at an earlier stage.

### Is keel the first Rust libc?

No. relibc, eyra, and anp/rusl came earlier. keel's contribution is the
combination of the musl 1.2.6 ABI, an integrated dynamic linker, three
architectures, and published real-workload and LTP validation.

### Is keel production-ready?

No. keel is `0.8.x` and experimental. See the [workload matrix](workloads.md)
and [LTP report](ltp.md) for the validated scope, and read
[Status and scope](../README.md#status-and-scope) before using it outside a
test environment.

### Is keel memory-safe?

No libc is entirely memory-safe in the Rust sense. keel uses `unsafe` at the
syscall and pointer boundary. Rust reduces the unsafe surface; it does not
eliminate it. keel has not had a formal security audit.

### Why is startup slower than musl?

The dynamic loader currently performs more work at startup than musl's. The
microbenchmark results and the startup caveat are documented in
[performance.md](performance.md).

### Can keel run glibc binaries?

No. keel implements the musl ABI. Running glibc-built binaries would require
a different ABI implementation.

## Validation

| Claim | Where to verify |
|---|---|
| Nine real Alpine workloads pass | [workloads.md](workloads.md) |
| LTP failure lists match musl | [ltp.md](ltp.md) |
| libc-test: 414 PASS / 5 SKIP / 0 FAIL | `libc-test-harness/reports/` |
| 0 unexpected parity diffs vs musl 1.2.6 | `scripts/parity-check.sh` |
| Full musl 1.2.6 symbol surface across `libc.so` + `libldso.so` | `scripts/parity-check.sh symbols` |
| x86\_64, aarch64, riscv64 | CI and `scripts/riscv-smoke.sh` |

## Reporting a compatibility issue

A useful report includes the program and version, the Alpine package name, the
exact command, the expected output, and the actual output. Open an issue with
those details; reproducible reports may be added to the workload matrix.
