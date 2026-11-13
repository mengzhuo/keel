# Agent Handoff

## Mission

`keel` is a Rust `no_std` musl-compatible libc plus a dynamic linker. Goal: run unmodified musl-linked x86_64/aarch64/riscv64 ELF binaries. Port behavior from `/home/root/musl`; validate against `/home/root/libc-test`.

## Repo Layout

| Path | What it is |
|------|------------|
| `src/` | `keel` binary (package `keel-rt`): maps a standalone PIE/static ELF and jumps to its entry. It is **not** the dynamic linker. |
| `libc/` | Package `keel-libc`; `[lib] name = "c"` produces the combined loader/libc `target/debug/libc.so` and `libc.a`. `_start` and the dynamic linker live in `libc/src/ldso/`. |
| `include/` | Hand-maintained C headers used by fixtures and packaged in `keel-dev`. |
| `crt/` | Rust `global_asm!` startup objects (`crt1.o`, `Scrt1.o`, `crti.o`, `crtn.o`); built by `scripts/build-crt.sh`. |
| `utils/` | Package `keel-utils`; Rust multicall binary for `getconf`, `getent`, `iconv`, `ldd`, `ldconfig`. |
| `tests/` | Root-package integration tests; C fixtures live in `tests/fixtures/`. |
| `libc-test-harness/` | Shell runner for upstream `libc-test`; reports in `libc-test-harness/reports/`. |
| `scripts/local-ci.sh` | Runs `cargo build/test --workspace` in the Ubuntu CI image (it does not run the harness). |
| `scripts/alpine-ci.sh` | Runs `cargo build/test --workspace` in latest Alpine with the nightly musl toolchain and Debian-compatible shims. |
| `scripts/riscv-smoke.sh` | Cross-builds and runs riscv64 fixtures under `qemu-riscv64`. |
| `scripts/bench.sh` | Compares keel with musl on microbenchmarks and startup. |

## Build And Test

**For local verification, run all tests in a container, not on the host.**
The host `target/` in this lab may still contain artifacts from the
pre-rename `/home/root/crabc` checkout, so `utils/tests/cli.rs` embeds the old
`CARGO_BIN_EXE_keel-utils` path and fails with `No such file or directory`.
Use `./scripts/local-ci.sh` for the Ubuntu CI image, or
`./scripts/alpine-ci.sh` for latest Alpine; if you reuse this checkout's
`target/`, run `cargo clean` first. The Ubuntu container needs `gcc`,
`musl-gcc`, `file`, and binutils (`readelf`/`objdump`), not just Rust. Run the
libc-test harness and parity scripts inside the container after
`cargo build --workspace`; `local-ci.sh` does not invoke them.

```bash
./scripts/local-ci.sh        # Ubuntu container: cargo build + cargo test --workspace
./scripts/alpine-ci.sh       # latest Alpine container with musl toolchain

# Inside the container:
cargo build --workspace      # required before tests: builds the standalone keel binary and libc.so/a
cargo test --workspace       # all integration tests, after the build above
cargo test --test math       # one test file, after the build above

./scripts/build-crt.sh       # Rust-generated startup objects (dist/crt/)
./scripts/build-utils.sh     # keel-linked Rust musl-utils replacement (dist/utils/)
./scripts/package-apk.sh     # keel, keel-dev and keel-utils APKs
./scripts/macro-parity.sh    # expanded-macro parity vs musl 1.2.6 (3 arches)
./scripts/workload-suites.sh # upstream CPython/Git/nginx suites vs musl (root + Docker)
```

- `scripts/macro-parity.sh` downloads musl 1.2.6 into `~/.cache/keel`, generates its headers for x86_64/aarch64/riscv64, and compares the expansion of every macro defined by both keel and musl in both C and C++. It requires `gcc`/`g++`, `aarch64-linux-gnu-gcc`/`g++` and `riscv64-linux-gnu-gcc`/`g++`. Intentional differences live in `tests/macro-parity.allowlist`; all current entries are keel's 64-bit x86_64 `long double` ABI (in `float.h` and `values.h`). `tests/macro-parity.coverage` pins the missing-ABI-macro count at zero, and `tests/header_rust_consistency.py` checks that every numeric header macro matches the corresponding Rust constant.

- **Bare `cargo build` only builds the root `keel-rt` package (the `keel` loader).** The root `Cargo.toml` is both workspace and package, so `libc.so`/`libc.a` are not rebuilt. Use `--workspace` or `-p keel-libc`.
- `-p libc` and `-p ldso` do not exist. The artifact name comes from `[lib] name = "c"`, not the package name.
- Even `cargo test --workspace` does not build `libc.so`; integration tests assert that file exists. Always run `cargo build --workspace` first.
- Tests and the harness hardcode `target/debug`; `cargo test --release` still looks for debug artifacts. Stick to the dev profile.
- Cargo stops at the first failing test binary; use `cargo test --workspace --no-fail-fast` when you need the full picture.
- Toolchain is nightly (`rust-toolchain.toml`) and requires `musl-gcc`. `static_hello` also needs host CRT objects under `/usr/lib/<arch>-linux-musl/`.
- Dev profile is intentional: `opt-level = 2`, `overflow-checks = false`, `panic = "abort"`. The release profile is size-oriented (`opt-level = "s"`, `strip = "symbols"`, `panic = "abort"`), and `.cargo/config.toml` sets `RUST_TEST_THREADS=1` plus `-C force-unwind-tables=no`. Do not change these without re-running the size and performance measurements.
- New fixture binaries are written into `tests/fixtures/`; add each output path to the manual list in `.gitignore`.
- `tests/compat.rs` checks `glob("/tmp/*")`; a fresh container has an empty `/tmp`. `local-ci.sh` seeds a file, but manual runs need `touch /tmp/keel-ci-seed` first.
- On Alpine, cargo drops the `cdylib` unless `RUSTFLAGS='-C target-feature=-crt-static'`; static fixtures also need `musl-gcc` to add `-no-pie` and `/usr/lib/<arch>-linux-musl/crt*.o` symlinks. `scripts/alpine-ci.sh` sets these up.
- If the container cannot reach the package/rustup registries, set `HTTP_PROXY`/`HTTPS_PROXY`; in this lab use `http://10.0.0.1:8123`. Both CI scripts forward them.

## Test Pattern

Most `tests/<name>.rs` (canonical: `tests/math.rs`) use `tests/common/mod.rs`:

1. `common::run_fixture(name, extra)` compiles `tests/fixtures/<name>_test.c` with
   `musl-gcc -fPIE -pie -I include/ ... -Wl,--dynamic-linker
   target/debug/libc.so -L target/debug -lc -Wl,--allow-shlib-undefined`;
2. it runs the binary with `LD_LIBRARY_PATH=target/debug`;
3. the test asserts exact stdout (for example `"math ok\n"`).

Pass extra compiler flags through the `extra` slice. Math adds
`-fno-builtin -frounding-math`; on x86_64 also `-mlong-double-64`.
`common::compile_static(src, bin)` builds a static fixture against `libc.a`
with the host musl CRT objects; tests then run it directly and through the
standalone `loader`. Tests that build DSOs or custom loader layouts still
invoke the toolchain directly. `tests/crypt.rs` writes its C source inline, so edit the Rust string, not `tests/fixtures/crypt_test.c`. `tests/wave4.rs` and `tests/wave5.rs` shell out to `libc-test-harness/run.sh` and assert aggregate PASS/FAIL counts; they are slow and depend on the harness setup. They skip when `LIBC_TEST_DIR` (default `/home/root/libc-test`) is absent, so set it to a nonexistent path when you only want the local fixtures.

`libc` sets `test = false`; all coverage is through these root integration tests.

## Adding A libc Function

1. Implement it in `libc/src/lib.rs` or a `libc/src/<area>.rs` file, porting musl's algorithm literally.
2. If the file is new, add `include!("<area>.rs");` in `libc/src/lib.rs` at the right point. Order matters (`math_helpers.rs` comes before the math implementations, and several includes are near the end of the file).
3. Add the prototype to the matching `include/*.h`; headers are hand-written.
4. Add coverage to `tests/fixtures/<area>_test.c` and the expected stdout in `tests/<area>.rs`.
5. Run `cargo build --workspace && cargo test --test <area>`, then `libc-test-harness/run.sh <subset>` for broader checking.

## Implementation Notes

- `libc/src/lib.rs` is a single large `no_std` file. Most subsystems are `include!`-ed; the dynamic linker is the normal `mod ldso`. Architecture selection, the `Syscalls` trait/implementations and the per-architecture syscall-number tables live in `libc/src/arch.rs`, `libc/src/ldso/arch.rs` and the standalone loader's `src/arch.rs` (the first two are `include!`-ed; the standalone loader uses a normal module). The compatibility surface is split across area files: `netdb_extra.rs`, `compat_extra.rs`, `stdio_extra.rs`, `ns_parse.rs`, `fts.rs`, `ifaddrs.rs`, `dl_extra.rs`, `path_extra.rs`, `wordexp.rs`, `des.rs`, and `getopt_impl.rs`.
- `libc/src/alloc.rs` implements `malloc`/`free`/`calloc`/`realloc` with 16-byte size classes from 32 to 32768 bytes, mmap-backed chunks, direct mmap for large blocks, and a global spin lock. `fork()` resets the lock in the child. Joinable thread slots are reused. The thread table starts with a static 1024-slot chunk and grows in linked 1024-slot mmap chunks; free slots form a linked list protected by a futex lock. `fork()` rebuilds that list for the child and keeps only the calling thread.
- The initial thread is registered directly in `THREADS[0]` by `__libc_start_main()`; do not call `pthread_self()` there. `alloc_thread_slot()` excludes slot 0 from the free list, and `thread_allocator_after_fork()` updates that slot without scanning the table when the free list has not been built. Scanning the 1024-slot `Thread` table faults in roughly 1 MiB and used to make process startup about 3x musl. `libc.so` is linked with `-Wl,-Bsymbolic-functions`; public data stays preemptible for COPY relocations, and `_start` self-relocates the RELA table before the loader runs.
- TCB offset `0x40` holds the current thread's `Thread *`. Every TCB constructor zeros it, `expand_thread_tls()` preserves it, and `thread_entry()` sets it for new threads. `pthread_self()`/`find_thread()` use it to avoid scanning the table, falling back to a scan only when the pointer is absent or stale. Like the getrandom slot, it is accessed with unaligned loads/stores because TP can be 4-byte aligned.
- TCB offset `0x20` (reserved in the static TLS layout) is the per-thread list of dynamic TLSDESC extension blocks. Modules loaded after startup get `__tlsdesc_dynamic`, whose descriptor argument packs `(module << 32) | offset`; the resolver lazily maps a per-thread copy of the module's `.tdata` so threads created before `dlopen` see the initial values and keep their own writes. Startup modules keep the static descriptor. Both resolvers must be assembly stubs that preserve every register except the return value: the aarch64 compiler keeps TP in `x2` across the call, and a normal Rust function clobbers it. On aarch64/riscv64 `__tls_get_addr` resolves post-startup modules from the same extension list so `dlsym` and TLSDESC share one copy; x86_64 has no TLSDESC and keeps using the expanded main block.
- `__tlsdesc_static` and `__tlsdesc_dynamic` are `.hidden`; `__keel_tlsdesc_dynamic` is the Rust helper. RISC-V's cross toolchain emits `R_RISCV_TLS_DTPMOD64`/`__tls_get_addr` rather than TLSDESC today, so the RISC-V dynamic stub is currently dormant but must stay in sync.
- `scripts/workload-suites.sh` builds two Alpine chroots (stock musl and keel), bind-mounts `/proc`, `/sys`, `/dev` and a shared work directory, and runs a curated CPython `test` list, Git's `t/` scripts and `nginx-tests`. It builds Git's `t/helper/test-tool` under keel. Reports go to `workload-suites/reports/`. It is not part of the release gate; run it manually or via the weekly `Workload suites` workflow. Known differences and current results are in `docs/workload-suites.md`.
- `pselect()` must pass `pselect6` a `{ sigmask, sigsetsize }` struct, not the raw sigset pointer. GNU make blocks `SIGCHLD` and relies on pselect's temporary mask, so getting this wrong deadlocks `make -j`. `regoff_t` must be `long` (musl's `_Addr`), not `int`: prebuilt BusyBox has a different `regmatch_t` layout otherwise. The regex engine's `.` and `+` handling and growable backtracking stack are covered by `tests/compat.rs`.
- Time-zone support lives in `libc/src/tz.rs`: TZif v1/v2/v3 files and POSIX `TZ` strings, with `localtime`/`mktime`/`tzset` using it. `year_to_secs` must compare `(year - 2) as u64 <= 136`, not a signed expression; negative years take the generic path. `ctime` uses `localtime`, and `asctime_r` formats the day as `%3d` and the year as signed `%d`.
- The dynamic loader rebuilds argv/envp without a fixed cap: a temporary pointer array is mmap'ed and unmapped before the jump. The loaded-object table keeps stable indices but grows by mmap'ing a larger array and copying; each object's dependency array and name are mmap-grown too. `dlopen` is limited only by address space and the 4 GiB DSO-base stride. `pthread_atfork` uses a growable `realloc` registry, so there is no 64-handler cap.
- Regex bytecode, bracket tables, per-bracket ranges and the match stack are realloc-grown (jump offsets are 32-bit). Passwd/group parsing uses `getline` plus growable field/member buffers. AIO completion records and wordexp word buffers are dynamically allocated. These follow musl's `getline`/realloc/dynamic-list approach rather than fixed tables.
- Internal path and line buffers use Linux's PATH_MAX (4096) or `getline`, including `dladdr`/`dlinfo` names, `/proc/self/maps`, `/proc/self/exe`, `fgetln` and `getmntent`. Public `limits.h` macros stay musl-compatible where musl differs from Linux (for example `HOST_NAME_MAX`, `ARG_MAX`, `NGROUPS_MAX`).
- `errno` is thread-local for dynamically linked programs: it lives at offset `0x30` in the 256-byte TCB that `ldso` allocates for every thread (`0x00` self pointer, `0x08` generation, `0x28` stack canary). `__libc_start_main` calls `errno_init()`; when no TCB is present (static executables), `errno` falls back to a process-global slot. Internal code must use the `ERRNO!()` accessor, never a direct static.
- `pthread_create()` carves the TLS block out of the top of an application-provided stack when it fits. This mirrors musl and keeps the loader's separate TLS mmap from landing directly below a `MAP_GROWSDOWN` stack, which would block stack growth (`LTP mmap18`). `__keel_init_thread_tls_at` in `libc.so` initializes the embedded block.
- `dlopen()` follows musl: `LoadedObject` tracks its PT_LOAD address range, direct `DT_NEEDED` indices, `needed_by` parent and RUNPATH/RPATH. `RTLD_NOLOAD`/`NODELETE`/`GLOBAL`/`LOCAL` and dependency-scope `dlsym` are implemented. The public `dlsym` is a tiny per-arch naked wrapper that passes the caller's return address to `__ldso_dlsym`, which is what makes `RTLD_NEXT` work. `dlclose` validates the handle but, like musl, never unloads modules or runs destructors.
- `keel-utils` is Rust, not a copy of Alpine's C utilities. It is a statically linked multicall binary for `getconf`/`getent`/`iconv`/`ldd`/`ldconfig`. `ldd` parses ELF directly; `ldconfig` reads SONAME entries directly.
- `keel-dev` ships the 224-header musl-dev-compatible set (217 musl-dev headers plus 7 keel extras), `libc.a` and Rust `global_asm!` CRT objects including `rcrt1.o`. Static PIE, static non-PIE, constructors, initial-exec TLS and static pthread TLS work; `init_static_tls()` builds a TCB when no dynamic loader is present, and `pthread_create()` allocates static thread TLS from the saved PT_TLS image.
- `poll()` on aarch64/riscv64 must convert the millisecond timeout to a `struct timespec` before calling `ppoll`; Rust std calls `poll(..., 0)` during startup and would otherwise block forever.
- The byte loops in `memset_words()` use `write_volatile`; without it LLVM recognises the loop as a `memset` idiom on aarch64 and emits a recursive call to `memset`, overflowing the stack for small unaligned fills (Rust `HashSet` hit this).
- `match_sym()` must reject `SHN_UNDEF` symbols. ET_EXEC objects can carry a non-zero `st_value` (the PLT stub) for an undefined function; resolving it makes the PLT jump to itself.
- `mkstemps()` must honour the suffix length; GCC's temp-file creation uses a template whose final six characters are not `XXXXXX`. `vfork()` intentionally forwards to `fork`; a clone-based vfork broke LTP `vfork01` and the mkfs helpers (child segfaults), while fork semantics pass.
- vDSO entry points are resolved from `AT_SYSINFO_EHDR` in `libc/src/vdso.rs`. `clock_gettime`, `gettimeofday`, `time`, `sched_getcpu` and `getrandom` use it; the real syscall is always the fallback. Both `__vdso_*` and `__kernel_*` names must be tried (aarch64 uses the latter). The `sched_getcpu` output slot is a separate 16-byte scratch area: a 4-byte local lands inside the compiler's `push` and the vDSO's 4-byte write then costs ~2.5 ns in store forwarding. Keep the syscall fallback `#[cold]`.
- vDSO `getrandom` uses a per-thread opaque state. Its pointer lives at TCB offset `0x38` (errno ends at `0x34`), so every TCB constructor must zero that slot, `expand_thread_tls()` must preserve it, and thread exit must `munmap` the state page. The page is allocated with the kernel-provided `MAP_DROPPABLE` flags and `MADV_WIPEONFORK`; without the fork wipe the vDSO must not be used, or parent and child could reuse the same buffered bytes. A DSO with 4-byte TLS alignment can leave the thread pointer only 4-byte aligned, so access the slot with `read_unaligned`/`write_unaligned`; an `&AtomicUsize` or `read_volatile` at that address is misaligned and panics in debug builds.
- The standalone `loader` rebuilds the target auxv. It must pass through `AT_SYSINFO_EHDR` and `AT_PHENT`, translate `AT_PHDR` from a file offset to its runtime address, extend PT_LOAD mappings past `p_filesz` for BSS, keep the final `sp` 16-byte aligned before writing `argc`, and clear its own thread pointer before jumping to a static target. `tests/static_hello.rs` and `tests/vdso.rs` cover the direct and loader paths.
- The `keel` runtime package installs the canonical `/lib/ld-musl-<arch>.so.1` and `/lib/libc.musl-<arch>.so.1` symlinks, provides `musl` and `so:libc.musl-<arch>.so.1`, and replaces `musl`; installing it purges the `musl` package record and switches the system libc.
- `keel-dev` provides both `libc-dev` and `musl-dev` and replaces `musl-dev`; installing it purges Alpine `musl-dev`, while `build-base`/`g++` dependents remain satisfied. `keel-utils` provides `libc-utils`, `musl-utils`, and `cmd:*`, and replaces `musl-utils`; installing it purges `musl-utils` and `scanelf`.
- When a new DSO is loaded, TLS layout offsets are assigned **before** relocations (aarch64 TLSDESC/TPOFF embed them) and the relocated `.tdata` image is copied into threads **after** relocations. Do not merge these steps: doing so either corrupts TLSDESC offsets or copies unrelocated pointers.
- The combined `libc`/`ldso` is `no_std`; only the standalone `src/` loader uses `std`.
- C ABI exports need `#[no_mangle] pub unsafe extern "C" fn ...`. A plain Rust function will not appear in `libc.so`/`libc.a`.
- `libm = "0.2"` is only a stopgap for functions not yet ported. When porting one, remove its `libm::` call.
- `long double`: keel's x86_64 ABI is 64-bit (fixtures pass `-mlong-double-64`); aarch64/riscv64 use `f128` signatures, though `math_long_double.rs` currently computes through `f64`.
- `libc/build.rs` links `libc.so` with `-Wl,-Bsymbolic-functions`; keep `_start` as the weak `global_asm!` entry in `libc/src/ldso/mod.rs`. `_start` applies the interpreter's RELATIVE relocations; the loader later resolves its non-RELATIVE entries and skips the RELATIVE ones. The interpreter registers itself as `libc.so` before dependencies so `DT_NEEDED libc.so` resolves to the existing mapping.
- CI (`.github/workflows/ci.yml`) runs build + `cargo test --workspace` natively on x86_64 and aarch64 (`ubuntu-24.04-arm`). It caches libc-test by upstream HEAD SHA and sets `LIBC_TEST_DIR`, so `wave4`/`wave5` run regression/functional inside `cargo test`; the API suite runs on both arches and the math suite on x86_64. The native riscv64 entry (RISE `ubuntu-24.04-riscv`) is currently commented out because the runner does not come up; the x86-hosted `riscv64-cross` job still cross-builds the musl target and runs the qemu smoke suite (non-blocking, 10 minute cap).
- `scripts/local-ci.sh` accepts `CI_RUNNER` (podman/docker) and `CI_IMAGE` overrides.
- All public musl function symbols are exported from the combined `libc.so`. Because musl's `libc.so` is also its loader, `scripts/parity-check.sh symbols` compares the two single-library symbol sets. `libc.so` exports a `__tls_get_addr` entry so C++ linkers can resolve `libstdc++.so`; the loader installs its multi-module resolver through `__keel_set_tls_get_addr` before constructors, and static executables use the module-0 fallback. The resolver (`res_*`) and utmp/utmpx are real implementations; `_init`, `_fini`, and `_dl_debug_*` are linker internals and are intentionally absent.
- `tests/compat.rs` exercises the non-libc-test ABI surface (resolver, netdb, passwd/group/shadow, utmp, AIO, glob/nftw/realpath, wordexp, fopencookie, DES crypt, dladdr, statfs, sysinfo, posix_getdents, pthread join variants, futimesat, mremap, etc.).

## Releasing

Releases are published by `.github/workflows/release.yml` on `v*` tags using crates.io Trusted Publishing (OIDC, no long-lived token). The job uses the GitHub environment `release`.

Release artifacts use the size-oriented profile. Static consumers should link with `-Wl,--gc-sections`; without it a Rust staticlib pulls in far more code.

One-time setup on crates.io, for **each** of `keel-rt`, `keel-libc`, and `keel-utils`:

- open the crate's **Settings → Trusted Publishing → Add a new Trusted Publisher**
- GitHub repository owner: `mengzhuo`
- GitHub repository name: `keel`
- Workflow filename: `release.yml`
- Environment: `release`

To cut a release, bump the version in `Cargo.toml`, `libc/Cargo.toml`, and `utils/Cargo.toml`, commit, then push a matching tag:

```bash
git tag vX.Y.Z
git push origin main vX.Y.Z
```

The workflow verifies that the three manifest versions equal the tag before publishing. It publishes `keel-libc`, `keel-rt` and `keel-utils`, and creates a GitHub release. Release artifacts include `keel`, `keel-dev` and `keel-utils` APKs plus `SHA256SUMS`; see `docs/packages.md`.

The same workflow builds release artifacts for x86_64, aarch64, and riscv64 and packages them with `scripts/package-apk.sh`. The riscv64 loader and `keel-utils` use `rust-lld` plus Rust's self-contained musl sysroot because that target lacks `crt-static-default`; the cross glibc compiler is not used for static binaries. The script runs `apk mkpkg` inside an Alpine 3.23 container (Docker or podman), producing v3/ADB packages, then writes `dist/SHA256SUMS`. The `.apk` files and checksums are uploaded to the GitHub release; they install with `apk add --allow-untrusted`.

x86_64 and aarch64 musl targets force `crt-static` and cannot produce a cdylib with the current toolchain, so their no_std libraries are built for the gnu target (they do not depend on the C library) while the loader is built for musl. riscv64 builds the whole workspace for the musl target.

If publishing manually, crates.io verifies each package standalone, where the workspace `panic = "abort"` is absent. Use an explicit abort strategy for the no_std crates:

```bash
RUSTFLAGS='-C panic=abort' cargo publish -p keel-libc --registry crates-io
cargo publish -p keel-rt --registry crates-io
cargo publish -p keel-utils --registry crates-io
```

The default registry in this environment is a mirror; pass `--registry crates-io` to publish to crates.io.

## riscv64

- Cross-build: `CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_MUSL_LINKER=riscv64-linux-gnu-gcc cargo build --workspace --target riscv64gc-unknown-linux-musl`.
- `scripts/riscv-smoke.sh` cross-builds if needed and runs hello, tiny-PIE/loader, TLS/DSO, math, pthread, socket, net, and other fixtures under `qemu-riscv64`. Requires `gcc-riscv64-linux-gnu` and `qemu-user`. On native riscv64 it uses `musl-gcc` and runs the fixtures directly (set `RISCV_CC`/`QEMU` to override).
- The combined `libc.so` uses RELA self-relocation; do not re-enable `-z pack-relative-relocs` until `_start` handles RELR and the loader can apply the interpreter's non-RELATIVE entries safely.
- `c_char` is `u8` on riscv64 (it is `i8` on x86_64). Use `b'x' as c_char`, not `'x' as i8`, in code compiled for both.
- `struct stat` is architecture-specific: x86_64 uses the older order; aarch64/riscv64 use the generic 128-byte layout. Keep Rust `Stat` and `include/sys/stat.h` in sync.
- `nlink_t` and `blksize_t` are 32-bit on aarch64/riscv64 but 64-bit on x86_64; see `include/sys/types.h`.

## libc-test Harness

Run these inside the test container, after `cargo build --workspace`.

```bash
cd libc-test-harness
./run.sh              # functional (default)
./run.sh math         # math subset
./run.sh regression
./run.sh api          # compile-only header/API checks
./run.sh all
```

- Defaults to `LIBC_TEST_DIR=/home/root/libc-test`; override it for another checkout.
- It symlinks `fake-libs/{libc,libpthread,libm,...}.so` to our `libc.so`, builds `runtest.exe`, and writes `reports/latest-summary.txt` / `latest-raw.txt`.
- When a test exits 0, any non-empty stderr line not starting with `ldso:` or `JUMP ` is still treated as FAIL.
- Its fallback build is bare `cargo build`, which will not build `libc.so`; run `cargo build --workspace` first.
- It creates symlinks in `$LIBC_TEST_DIR/src/<subset>/`, so it mutates the external libc-test checkout.
- Run one subset at a time. All subsets share `build/`, `fake-libs/`, and `reports/`; concurrent runs race and corrupt counts.
- `math` intentionally SKIPs `acoshl`, `asinhl`, `lgammal`, and `tgammal`: upstream uses 80-bit long-double vectors while keel's x86_64 ABI is 64-bit. This is recorded in `SKIP_TESTS` in `run.sh`.
- As root, `regression/pthread_atfork-errno-clobber` is also skipped: `RLIMIT_NPROC` is not enforced for root, so musl itself succeeds at `fork()` in that environment.

## musl Parity Check

`scripts/parity-check.sh` checks behaviour against real musl, not just against
libc-test's assertions. It compiles each libc-test source twice against the
same musl libc; the two binaries differ only in their ELF interpreter:

- `musl` runs with `$MUSL_LIBC` (the real loader/libc);
- `keel` runs with `target/debug/libc.so` and `LD_LIBRARY_PATH=target/debug`.

It compares exit status, stdout and stderr, retries timing-sensitive tests,
and skips the x86_64 long-double ABI tests.

```bash
# Build a matching baseline once. keel ports musl 1.2.6 behaviour.
(cd /path/to/musl-1.2.6 && ./configure && make -j)
MUSL_LIBC=/path/to/musl-1.2.6/lib/libc.so scripts/parity-check.sh all
MUSL_LIBC=/path/to/musl-1.2.6/lib/libc.so scripts/parity-check.sh symbols
```

- Use musl 1.2.6. An older system musl reports false differences (for example
  `strptime` `%F`/`%z` and the default `getmntent` fields).
- `KNOWN_DIFFS` records the deliberate `strptime` `%s`/`%z` extensions and the
  exact `x^1`/`x^-1` identities in `pow`/`powf` (they avoid musl's spurious
  INEXACT/UNDERFLOW flags). All other math functions are ported from musl in
  `math_exp.rs`, `math_erf.rs`, `math_lgamma.rs`, `math_tgamma.rs`, and
  `math_bessel.rs`.
- `symbols` compares exported dynamic symbol sets; only `_init`, `_fini`,
  `_dl_debug_addr`, and `_dl_debug_state` are absent from `libc.so` as linker
  internals.

## Performance

`scripts/bench.sh` compares keel against a musl 1.2.6 baseline using the same
binary-swap method as the parity check, plus a fork/exec startup benchmark.
Results and methodology are in `docs/performance.md`. Use `BENCH_CPU=0` to pin
the measurement and `ROUNDS=5` (the default) for a median of interleaved
rounds.

## Current Baseline

Baseline (2026-11-10 full run; container `cargo test --workspace` and header
count rechecked 2026-11-13):

```text
cargo test --workspace          # green in clean Ubuntu and Alpine containers
scripts/alpine-ci.sh            # Alpine 3.23.3, cargo test green
./run.sh api                    # 79/79
./run.sh functional             # 73/73
./run.sh regression             # 67 PASS, 1 SKIP, 0 FAIL (as root)
./run.sh math                   # 195 PASS, 4 SKIP, 0 FAIL
scripts/parity-check.sh all     # 0 unexpected diffs vs musl 1.2.6
scripts/riscv-smoke.sh          # riscv64 cross-build + qemu fixtures, 0 failures
scripts/macro-parity.sh         # 0 mismatches, 0 missing ABI macros (3 arches, C/C++)
tests/header_rust_consistency.py # 1385/1385 numeric header macros match Rust

LTP 20250930, x86_64, 1488 syscalls tags:
  musl baseline                 # 112 FAIL, 139 TCONF
  keel                         # 102 FAIL, 140 TCONF
  per-test diff                 # no keel-only failures
math / mm / sched / nptl / ipc  # failure lists identical to musl
```

- `scripts/parity-check.sh` needs an x86_64 musl 1.2.6 build; the
  `/home/root/musl` checkout in this lab is riscv64. Build an x86_64 tree
  under `/tmp/opencode` and point `MUSL_LIBC` at its `lib/libc.so`. Passing
  the riscv64 `libc.so` produces spurious "corrupted shared library" diffs.
- LTP details, setup, and caveats are in `docs/ltp.md`. The full LTP tree must
  be built and installed explicitly; a bare `make -k` leaves about 110
  syscall binaries unbuilt after the `timer_create` failure. Verify with
  `grep -c "execvp of" /tmp/ltp-*.out` (must be 0).

- CI runs as an unprivileged user, so `pthread_atfork-errno-clobber` runs and
  passes there: `415 PASS, 4 SKIP`.
- If a run shows a different result, first re-run sequentially and compare against the previous report in `libc-test-harness/reports/`.
- The four math SKIPs are the only known expected non-passes; do not "fix" them by weakening the implementation.
