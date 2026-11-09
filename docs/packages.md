# Packages

crabc's Alpine packages mirror Alpine's `musl`, `musl-dev` and `musl-utils`
split:

| Package | Contents | Installed size |
|---|---|---|
| `crabc` | `libc.so`, `libldso.so`, `crabc-loader` | about 1.8 MiB |
| `crabc-dev` | headers, `libc.a`, Rust-built CRT objects, linker stubs | about 10 MiB |
| `crabc-utils` | `getconf`, `getent`, `iconv`, `ldd`, `ldconfig` | about 1.9 MiB |

All three are built for `x86_64`, `aarch64` and `riscv64`.

## crabc

Installs the runtime libraries under `/usr/lib/crabc`:

```text
/usr/lib/crabc/libc.so
/usr/lib/crabc/libldso.so
/usr/lib/crabc/libc.musl-<arch>.so.1 -> libc.so
/usr/bin/crabc-loader
```

The `/usr/lib/crabc/libc.musl-<arch>.so.1` alias lets third-party DSOs that
record the musl SONAME (notably `libgcc_s.so.1`) resolve to crabc libc.

`crabc-loader` is the standalone mapper for static PIE and static ELF
binaries.  Dynamic binaries use `libldso.so` as their ELF interpreter.

The package also installs symlinks at the canonical musl paths:

```text
/lib/ld-musl-<arch>.so.1 -> /usr/lib/crabc/libldso.so
/lib/libc.musl-<arch>.so.1 -> /usr/lib/crabc/libc.so
```

It declares `replaces: musl`, so `apk add crabc` switches the system libc
without removing the `musl` package record.  Do not run `apk del musl` or
`apk fix musl` while crabc owns those paths.

## crabc-dev

Development files mirror `musl-dev`:

```text
/usr/include/...            217 musl-dev-compatible headers plus crabc extras
/usr/lib/libc.a             crabc static libc
/usr/lib/crt1.o             non-PIE startup object
/usr/lib/Scrt1.o            PIE startup object
/usr/lib/rcrt1.o            static-PIE self-relocating startup object
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

Static PIE is supported through the Rust-built `rcrt1.o`; static linking
works with either `-static` (static PIE) or `-static -no-pie`.  Static
constructors and initial-exec TLS are initialized by the libc startup path.

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

The binary is statically linked, so the tools do not depend on a dynamic
loader, libc or `libgcc_s`.  All three architectures use the self-contained
musl sysroot (riscv64 goes through `rust-lld` because the target has no
`crt-static-default`).  `ldd` still inspects crabc and musl binaries by
parsing their ELF files.

`ldd` parses ELF files directly because crabc's loader is not a command-line
interpreter; it prints dependency paths but not load addresses.  `ldconfig`
reads SONAME entries directly and does not require `scanelf`.

The package declares the `libc-utils` and `cmd:*` virtuals and uses
`replaces: musl-utils`.  Alpine's solver treats those virtuals as exclusive
with `musl-utils`, so remove `musl-utils` first on a stock image:

```sh
apk del musl-utils
apk add --allow-untrusted crabc-utils-<version>-r0.<arch>.apk
```

The Dockerfile performs that removal automatically.

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

`scripts/build-utils.sh` builds a static musl multicall binary for each
target; riscv64 uses `rust-lld` and Rust's self-contained musl sysroot.
