# libc-test Integration Harness

Lives under `scripts/`. Builds and runs the [libc-test](https://git.musl-libc.org/cgit/libc-test) suite against keel's libc.so to produce a categorized failure report.

## How It Works

1. Builds `keel/libc` into `target/debug/libc.so`, running `cargo build --workspace` if it is missing (a bare `cargo build` would only build the loader).
2. Creates a `fake-libs/` directory with symlinks so the linker resolves `-lc`, `-lpthread`, `-lm`, etc. against our libc.so instead of musl's.
3. Builds libc-test's `runtest.exe` (statically linked against musl) and `libtest.a` as host tools.
4. Compiles and links each test binary with `musl-gcc -L fake-libs/`, then runs it via `LD_LIBRARY_PATH=fake-libs/`.
5. Categorizes results: **PASS** (including compile-only `api` checks), **FAIL**, **BUILDERROR** (compile/link failure), **SKIP** (known long-double, TLS-alignment and root-only differences), **TIMEOUT** (30s).

## Usage

```bash
./run.sh              # native functional tests (default)
./run.sh math         # native math tests
./run.sh regression   # native regression tests
./run.sh api          # native API/header tests
./run.sh all          # native, all categories

./cross-run.sh aarch64 functional
./cross-run.sh riscv64 regression
./cross-run.sh powerpc64le api
./cross-run.sh loongarch64 functional
```

Reports are saved to `reports/`. Symlinks `reports/latest-summary.txt` and `reports/latest-raw.txt` always point to the most recent run.

## Requirements

Native:

- musl-gcc (`apt install musl-tools`)
- Rust stable toolchain (for building keel)
- libc-test source in a sibling `libc-test/` checkout (override with the `LIBC_TEST_DIR` env var)

Cross (`cross-run.sh`):

- the target cross GCC and its `libc6-dev-*-cross` static libraries
- the matching Rust `-gnu` target
- `qemu-user`
- e.g. `apt install gcc-aarch64-linux-gnu libc6-dev-arm64-cross qemu-user`

`cross-run.sh` builds the test driver as a static cross-glibc binary and builds the test binaries against keel's headers/libc, then runs both through qemu.  `CROSS_CC`/`CROSS_QEMU`/`LIBC_TEST_DIR` override the defaults.

## Known Limitations

- **Run subsets sequentially** — `build/`, `fake-libs/`, and `reports/` are shared, so concurrent runs race.
- **Long-double math tests:** `acoshl`, `asinhl`, `lgammal`, and `tgammal` are skipped because upstream uses 80-bit vectors while keel's x86_64 `long double` ABI is 64-bit.
- **`dlopen` follows musl semantics.** `dlopen`, `dlsym`, `dlerror`,
  `dladdr`, `dl_iterate_phdr`, `RTLD_LOCAL`/`GLOBAL` (including the dependency
  closure), `RTLD_NOLOAD`, `RTLD_NODELETE`, `RTLD_NEXT`, `dlopen(NULL)`,
  dependency-scope `dlsym`, TLS symbols, `RUNPATH`/`$ORIGIN` dependency
  lookup and constructors are implemented and covered by libc-test plus the
  Python/Node workloads. Like musl, `dlclose` only validates the handle and
  never unloads a module or runs `DT_FINI`/`DT_FINI_ARRAY` destructors;
  `RTLD_DEEPBIND` is not part of musl's API and is not defined. `RTLD_LAZY`
  is accepted but relocations are resolved eagerly.
- **Static linking is not tested by this harness** — only dynamic-linked
  binaries are built here; the root integration tests cover `libc.a`.
- **Only the functional subset runs by default** — use `./run.sh all` for everything.

Cross baseline (qemu, pinned `libc-test` checkout): aarch64, riscv64 and
powerpc64le report `functional` 69 PASS + 3 SKIP; loongarch64 reports
68 PASS + 4 SKIP.  All four report `regression` 64 PASS + 4 SKIP as root
(65 + 3 unprivileged) and `api` 79/79.  The cross skips are the
robust-futex tests and `raise-race` (qemu-user's real-time signal/fork
behavior), `strtold`/`tls_local_exec` on the 128-bit
long-double/TLS-alignment targets, and `functional/stat` on loongarch64:
QEMU 10.2 writes the 32-bit `stat64` layout for `newfstatat` (the 64-bit
ABI struct is correct on real hardware).  `functional/tls_init_dlopen` is
retried up to five times because ppc64le TLS initialization in a freshly
dlopen'ed module occasionally segfaults under qemu.  Cross `math` is not
part of the gate; the x86_64 math suite below still runs natively.

Current native baseline (x86_64, pinned `libc-test` checkout): `functional` 72/72,
`regression` 66 PASS + 2 SKIP, `api` 79/79, `math` 195 PASS + 4 SKIP. The
current upstream checkout adds `functional/mntent`, so a local run reports
73/73; CI keeps the pinned checkout so the baseline stays reproducible. The
regression skip is `pthread_atfork-errno-clobber`, which can only pass as an
unprivileged user and also fails under musl when root. On aarch64,
`functional/strtold` and `functional/tls_local_exec` are skipped for the
known long-double/TLS-alignment gaps. Counts move when the external
`libc-test` checkout changes.
