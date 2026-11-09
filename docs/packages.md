# Packages

crabc's Alpine packages mirror Alpine's `musl`, `musl-dev` and `musl-utils`
split:

| Package | Contents | Installed size |
|---|---|---|
| `crabc` | `libc.so`, `libldso.so`, `crabc-loader` | about 1.8 MiB |
| `crabc-dev` | headers, `libc.a`, Rust-built CRT objects, linker stubs | about 10 MiB |
| `crabc-utils` | `getconf`, `getent`, `iconv`, `ldd`, `ldconfig` | about 2 MiB with libgcc |

All three are built for `x86_64`, `aarch64` and `riscv64`.

## crabc

Installs the runtime libraries under `/usr/lib/crabc`:

```text
/usr/lib/crabc/libc.so
/usr/lib/crabc/libldso.so
/usr/lib/crabc/libc.musl-<arch>.so.1 -> libc.so
/usr/bin/crabc-loader
```

The `libc.musl-<arch>.so.1` alias lets third-party DSOs that record the
musl SONAME (notably `libgcc_s.so.1`) resolve to crabc libc.  It is not a
replacement for `/lib/libc.musl-<arch>.so.1`; the Docker image still installs
crabc at the canonical paths.

`crabc-loader` is the standalone mapper for static PIE and static ELF
binaries.  Dynamic binaries use `libldso.so` as their ELF interpreter.

The package deliberately does not overwrite `/lib/ld-musl-<arch>.so.1` or
`/lib/libc.musl-<arch>.so.1`.  The Docker image replaces those paths in
place; on a normal Alpine system use `LD_LIBRARY_PATH=/usr/lib/crabc` or link
with the crabc interpreter explicitly.

## crabc-dev

Development files mirror `musl-dev`:

```text
/usr/include/...            hand-maintained crabc headers
/usr/lib/libc.a             crabc static libc
/usr/lib/crt1.o             non-PIE startup object
/usr/lib/Scrt1.o            PIE startup object
/usr/lib/crti.o             .init/.fini prologue
/usr/lib/crtn.o             .init/.fini epilogue
/usr/lib/libc.so -> /usr/lib/crabc/libc.so
/usr/lib/libm.a             empty archives, because musl/crabc keeps
/usr/lib/libpthread.a       these symbols in libc
...
```

The CRT objects are generated from Rust `global_asm!` sources in `crt/` by
`scripts/build-crt.sh`; no musl C or musl object files are copied.  The
`libc.so` symlink lets `-lc` select crabc libc when `crabc-dev` is installed.

Static PIE (`rcrt1.o`) is not provided yet.  Static links work with
`-no-pie`; fully static PIE needs a self-relocating startup object that
crabc does not ship.

`crabc-dev` declares `provides: libc-dev` and `replaces: musl-dev`, so it is
a replacement for `musl-dev` rather than a package that can coexist with it.
If `build-base` is present, remove `musl-dev` first; installing `gcc` plus
`crabc-dev` is sufficient.

## crabc-utils

`crabc-utils` is one Rust multicall binary, `/usr/bin/crabc-utils`, with
symlinks matching `musl-utils`:

```text
/usr/bin/getconf
/usr/bin/getent
/usr/bin/iconv
/usr/bin/ldd
/sbin/ldconfig
```

The binary is dynamically linked against crabc libc and loader:

```text
interpreter: /usr/lib/crabc/libldso.so
NEEDED:      libc.so, libgcc_s.so.1
RPATH:       /usr/lib/crabc
```

`DT_RPATH` (rather than `DT_RUNPATH`) is intentional: it is inherited by
transitive dependencies, so `libgcc_s.so.1`'s `libc.musl-<arch>.so.1` lookup
also resolves to crabc.

`libgcc_s.so.1` is Rust's unwinder and comes from Alpine's `libgcc` package
(declared as a dependency).  `ldd` parses ELF files directly because crabc's
loader is not a command-line interpreter; it prints dependency paths but
not load addresses.  `ldconfig` reads SONAME entries directly and does not
require `scanelf`.

The package uses `replaces: musl-utils` so it can overwrite the commands in
Alpine's base image.  It does not declare the `libc-utils`/`cmd:*` virtuals,
because apk treats those as exclusive with `musl-utils`; declaring them would
make `apk add crabc-utils` fail on a stock Alpine image where `musl-utils`
is already installed.

## Building packages

```sh
cargo build --release -p crabc-libc -p crabc-ldso --target x86_64-unknown-linux-gnu
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
  cargo build --release -p crabc --target x86_64-unknown-linux-musl
# ... aarch64 and riscv64 builds ...

./scripts/build-crt.sh
./scripts/build-utils.sh
./scripts/package-apk.sh
```

`scripts/build-utils.sh` extracts `libgcc_s.so.1` from Alpine's `libgcc`
package with Docker when `LIBGCC_DIR_*` is not set.
