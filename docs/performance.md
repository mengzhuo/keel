# Performance

This is a microbenchmark comparison between keel and musl 1.2.6. The numbers
are a snapshot from one machine, not an application benchmark. Use the ratios
and the reproduction scripts rather than the absolute values.

## Method

- Host (x86\_64): AMD EPYC 7532, Linux 6.12, 64 available CPUs.
- `keel` release artifacts use the size-oriented release profile:
  `opt-level = "s"`, `strip = "symbols"`, `panic = "abort"`, and no unwind
  tables. This costs a few percent on stdio/math versus `-O3` and shrinks
  `libc.so` by about 30%.
- Baseline: musl 1.2.6 built from the pristine release tarball, used as the
  ELF interpreter/libc.
- The same C source is compiled twice against musl. The two binaries differ
  only in their ELF interpreter:
  - musl: `musl-1.2.6/lib/libc.so`
  - keel: `target/release/libc.so` with
    `LD_LIBRARY_PATH=target/release`
- Microbenchmarks: `clock_gettime(CLOCK_MONOTONIC)` around a batch, `-O2
  -fno-builtin`, so calls go through the libc. The vDSO group also measures
  `gettimeofday`, `time`, `sched_getcpu` and 32-byte `getrandom`.
- Startup: `fork` + `exec` + `wait` of a minimal dynamically linked program.
- Interleaved rounds, pinned to one CPU; the median is reported. The host is
  shared, so absolute values can move by 10-20% between sessions.

## x86\_64 results

`keel/musl` below 1.0 means keel is faster.

| Benchmark | musl | keel | keel/musl |
|---|---:|---:|---:|
| `memcpy` 64B | 10.8 ns | 6.7 ns | 0.62x |
| `memcpy` 4KB | 59.0 ns | 44.7 ns | 0.76x |
| `memset` 4KB | 63.3 ns | 50.2 ns | 0.79x |
| `memcmp` 4KB | 865 ns | 45.8 ns | 0.05x |
| `strlen` 4KB | 316 ns | 55.4 ns | 0.17x |
| `strcmp` (short) | 10.2 ns | 4.8 ns | 0.47x |
| `strstr` | 57.7 ns | 8.9 ns | 0.15x |
| `snprintf` | 434 ns | 260 ns | 0.60x |
| `fprintf` → `/dev/null` | 414 ns | 243 ns | 0.59x |
| `malloc`/`free` 16B | 104 ns | 13.4 ns | 0.13x |
| `malloc`/`free` 256B | 108 ns | 13.2 ns | 0.12x |
| `calloc` 1KB | 173 ns | 26.3 ns | 0.15x |
| `realloc` (grow) | 23.9 ns | 8.9 ns | 0.37x |
| `sin` | 5.19 ns | 5.02 ns | 0.97x |
| `exp` | 6.58 ns | 6.53 ns | 0.99x |
| `log` | 8.59 ns | 8.54 ns | 1.00x |
| `pow` | 21.8 ns | 23.9 ns | 1.10x |
| `sqrt` | 2.85 ns | 3.08 ns | 1.08x |
| `getpid` | 183 ns | 184 ns | 1.01x |
| `clock_gettime` | 27.7 ns | 27.8 ns | 1.01x |
| `gettimeofday` | 33.1 ns | 32.7 ns | 0.99x |
| `time` | 26.8 ns | 27.7 ns | 1.03x |
| `sched_getcpu` | 4.26 ns | 4.26 ns | 1.00x |
| `getrandom` 32B | 480 ns | 94.6 ns | 0.20x |
| `open`+`close` | 3468 ns | 3399 ns | 0.98x |
| `stat` | 1463 ns | 1456 ns | 1.00x |
| `getenv` | 88.6 ns | 37.0 ns | 0.42x |
| `pthread_mutex` lock/unlock | 12.3 ns | 6.5 ns | 0.52x |
| `pthread_create`+`join` | 55.7 µs | 62.7 µs | 1.13x |
| Process startup | 555 µs | 596 µs | 1.07x |

Geometric mean over the 30 benchmarks: **0.53x musl's runtime**.

The thread benchmark creates and joins 2000 threads; all 2000 ran in every
keel round.

## riscv64 results

Cross-compiled with `riscv64-linux-gnu-gcc` and run under `qemu-riscv64`.
Both binaries use the same crt objects and differ only in libc/interpreter.
QEMU/TCG numbers are not hardware numbers; only the ratios are meaningful.

| Benchmark | keel/musl |
|---|---:|
| `memcpy` 4KB | 0.53x |
| `memset` 4KB | 1.01x |
| `memcmp` 4KB | 0.43x |
| `strlen` 4KB | 0.96x |
| `strcmp` (short) | 0.58x |
| `malloc`/`free` 16B | 0.11x |
| `calloc` 1KB | 0.36x |
| `clock_gettime` | 1.06x |

Geometric mean: **0.52x**.

## Binary size

Release artifacts are stripped. musl's dynamic linker is its libc, so its
runtime footprint is one file; keel now also uses one combined `libc.so`.

| | x86\_64 | aarch64 | riscv64 |
|---|---:|---:|---:|
| `libc.so` | ~1.85 MB | ~1.9 MB | ~1.85 MB |
| Runtime total | **~1.85 MB** | **~1.9 MB** | **~1.85 MB** |
| musl `libc.so` | 666 KB | — | — |
| Alpine `.apk` | not measured | not measured | not measured |

For comparison, the historical 0.4.2 packages were 1.40 MB / 1.41 MB / 1.32 MB.

Static links use `libc.a` (9.7 MB as a Rust archive). Link with
`-Wl,--gc-sections`: a static hello is 26.5 KB versus musl's 17.8 KB, but
without `--gc-sections` it grows to 1.23 MB.

## What changed

- **SIMD string/memory paths.** `strlen`, `memcpy`, `memset`, `memcmp`,
  `memmove` and `strcmp` were rewritten. x86\_64 uses SSE2 (part of the
  baseline) with a runtime-dispatched AVX2 path; aarch64/riscv64 use word
  loops. The riscv64 path uses single `ld`/`sd` instructions (LLVM otherwise
  splits unaligned 64-bit accesses into bytes) and offset-addressed 32B
  blocks.
- **vDSO fast paths.** `clock_gettime`, `gettimeofday`, `time`,
  `sched_getcpu` and `getrandom` resolve their entry points from
  `AT_SYSINFO_EHDR` and call the kernel implementation directly; the syscall
  remains the fallback.  On Linux 6.11+ `getrandom` keeps a per-thread
  ChaCha20 state allocated with the kernel-provided `MAP_DROPPABLE` flags and
  `MADV_WIPEONFORK`, so forked children reseed instead of reusing the parent's
  buffered bytes.  This makes 32-byte `getrandom` about 5x faster than the
  syscall.  Both vDSO symbol naming schemes (`__vdso_*` and `__kernel_*`) are
  handled, and the loader preserves `AT_SYSINFO_EHDR` for static targets.
- **Packed relative relocations.** `libc.so` is linked with
  `-z pack-relative-relocs`; the loader applies both `DT_RELA` and `DT_RELR`.
- **Dynamic linker.** Symbol lookup uses each object's GNU/SysV hash table
  instead of a linear walk. The loader uses the kernel's existing `AT_PHDR`
  executable mapping instead of re-reading `/proc/self/exe` and remapping
  every segment, defers `$ORIGIN` until a RUNPATH needs it, and no longer
  processes its own already-applied relocations.
- **Symbol visibility.** The interpreter used to export its private
  `memcpy`/`strlen`/... helpers, so programs resolved those names to the
  loader instead of libc. The loader is now registered last in the symbol
  search order.
- **`log` / `pow`.** The near-1 and special-case branches are out of line,
  keeping the common paths compact.
- **Release profile.** `opt-level = "s"` with `strip` and no unwind tables
  trades a few percent of stdio/math throughput for a much smaller package;
  the previous `-O3` build was about 2.3x musl's size, the current one is
  1.06x.

## vDSO coverage

The vDSO is resolved lazily from `AT_SYSINFO_EHDR`; no syscall is needed on
the fast path.  Both symbol naming schemes are accepted because the kernel
uses `__vdso_*` on x86\_64/riscv64 and `__kernel_*` on aarch64.

| Function | Fast path | Fallback |
|---|---|---|
| `clock_gettime` | `__vdso_clock_gettime` / `__kernel_clock_gettime` | `clock_gettime` syscall |
| `gettimeofday`, `time` | built on `clock_gettime` | same |
| `sched_getcpu` | `__vdso_getcpu` / `__kernel_getcpu` (x86\_64, riscv64) | `getcpu` syscall |
| `getrandom`, `getentropy` | `__vdso_getrandom` / `__kernel_getrandom` (Linux 6.11+) | `getrandom` syscall |

vDSO `getrandom` needs an opaque per-thread state.  keel queries the kernel
for its size and mmap flags, maps one page per thread, and applies
`MADV_WIPEONFORK`.  The state pointer lives in the otherwise unused TCB byte
range at `0x38`, is preserved when `dlopen` grows a thread's TLS block, and is
unmapped when the thread exits.  Kernels without the symbol, or where the
state cannot be made fork-safe, use the syscall.

The standalone loader rebuilds the auxiliary vector for the target.  It
preserves `AT_SYSINFO_EHDR` and `AT_PHENT`, translates `AT_PHDR` from a file
offset to its runtime address, maps BSS past `p_filesz`, and clears its own
thread pointer before jumping to a static target.

## Remaining gaps

- **Process startup, 1.07x.** keel now uses one combined `libc.so` as both
  loader and libc, matching musl's single-file mapping.  `_start` self-relocates
  the RELA table; the loader then resolves the interpreter's remaining
  non-RELATIVE entries and skips the RELATIVE ones.  `libc.so` is linked with
  `-Bsymbolic-functions`, leaving public data preemptible for COPY relocations.
  The initial thread is registered directly in slot 0 instead of scanning the
  1024-slot thread table (that scan alone cost about 1 MiB of page faults per
  process).  The remaining difference is mostly allocator/stdio initialization
  and small loader mappings.
- **`pow`, 1.10x.** The port keeps exact `x^1` / `x^-1` identities (see
  `AGENTS.md`).
- **`pthread_create`+`join`, 1.13x.** The thread stack and TLS mappings
  dominate this microbenchmark; the host is shared and the ratio moves
  between sessions.

## Reproduce

x86\_64:

```bash
# Build musl 1.2.6 once, then:
MUSL_LIBC=/path/to/musl-1.2.6/lib/libc.so \
BENCH_CPU=0 ROUNDS=5 scripts/bench.sh
```

riscv64 (needs `gcc-riscv64-linux-gnu` and `qemu-user`):

```bash
MUSL_RISCV=/path/to/riscv64/musl/lib scripts/bench-riscv.sh
```

The scripts write raw results to `target/bench/` and
`target/bench-riscv/`.
