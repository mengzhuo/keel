# Known limitations

keel implements the musl 1.2.6 public symbol surface, but it is still
experimental (`0.1.x`).  This page records behaviours that are intentionally
different from musl or currently simplified, so they are not mistaken for
regressions.  It is not an exhaustive security review.

## stdio and formatting

- **printf width and precision are capped.**  To keep the formatter's stack
  buffers bounded, width is capped at 4096, integer precision at 1022,
  `%f`/`%a` precision at 1099, and `%e`/`%g` precision at 18.  Larger values
  are clamped; `snprintf` counts the clamped output.  This prevents stack
  overflows on attacker-controlled `*` width/precision, but differs from C
  for extreme values.

## Port status

- **powerpc64le and loongarch64 are bring-up targets.**  Both build a
  working `libc.so`, startup objects and standalone loader, and pass the
  `scripts/cross-smoke.sh` fixture set under qemu-user (including pthreads,
  TLS, `dlopen`, `setjmp`, fenv and math).  They have not been validated on
  native hardware, and the release workflow and APK packaging still list
  only x86\_64, aarch64 and riscv64.  `libc.so` for these targets is linked
  with the cross GCC driver because the stable musl Rust targets cannot emit
  a cdylib.
- **powerpc64le follows ELFv2's `TP_OFFSET`/`DTP_OFFSET` bias.**  The thread
  pointer sits 0x7000 bytes above the TLS variable area and DTP-relative
  pointers carry an 0x8000 bias, matching the ABI the static linker assumes.

## Threading

- **initial-exec TLS in a `dlopen`ed module is only reliable for threads
  created after the load.**  New modules are appended to the static TLS
  block and existing threads grow their block lazily on the next dynamic
  TLS access.  Global-dynamic and TLSDESC accesses use per-thread copies
  and work in threads created before the `dlopen`; an `initial-exec` access
  from such a thread can still use the old layout.  musl avoids this by
  reserving a fixed static TLS surplus at thread creation.
- **`getaddrinfo` and `getnameinfo` serialize on a resolver lock.**  The
  public APIs are thread-safe, but concurrent calls are serialized while
  they use the legacy non-reentrant `/etc/hosts` and service helpers.
- **stdio locking is caller-managed.**  `fgetc`, `fputc`, `fread`, `fwrite`
  and the other buffered I/O functions neither take nor honor the `FILE`
  lock.  `flockfile`/`funlockfile` exist as ABI entry points, but they do
  not protect against these functions, so applications must serialize a
  shared stream with their own mutex.
- **`random()` state is not locked.**  `random`, `srandom`, `initstate` and
  `setstate` share process-global state without a lock; musl serializes the
  same state.  Callers must serialize these functions themselves.
  `lrand48()` and the rest of the `*rand48` family are process-global in
  musl too and have the same caller-serialization requirement.

## Locale and wide characters

- **Locale data is minimal.**  `setlocale` recognizes `C`, `POSIX` and a
  UTF-8 ctype mode; there is no locale database, collation data, monetary
  formatting or localized time formatting beyond the C locale.
- **`iconv` supports a fixed encoding set.**  UTF-8/16/32, ASCII, the
  ISO-8859 family, Windows-1251/1252, KOI8-R, GBK/GB2312, Big5, EUC-JP and
  Shift-JIS are implemented; other musl charmaps are not available.

## Process and file APIs

- **The base page size is hardcoded to 4096.**  `getpagesize` and
  `sysconf(_SC_PAGE_SIZE)` do not query `AT_PAGESZ`, so they are wrong on
  aarch64/riscv64 kernels configured with 16 KiB or 64 KiB pages.
- **`fnmatch` rejects long components.**  Path components longer than 1024
  bytes return `FNM_NOMATCH` instead of being matched with a heap buffer.
- **`tsearch` is unbalanced.**  The search tree is a plain BST, so lookups
  and inserts are O(n) in the worst case; musl uses an AVL tree.
- **`wordexp` supports a subset of shell parameter expansion.**  `$VAR`,
  `${VAR}`, `$$`, command substitution and globbing work; `${VAR:-word}`,
  `${VAR:+word}`, `$?` and positional parameters are not expanded.

## Loader

- **The standalone `keel` mapper drops the environment.**  It builds an
  `argv` and auxiliary vector for the target but gives it an empty `envp`.
  Dynamic binaries using `libc.so` as the interpreter are not affected.
- **The standalone mapper shares ELF parsing, mapping and startup with the
  dynamic linker, but not symbol relocation.**  Static executables have no
  dynamic symbol table, so the mapper only applies RELATIVE and RELR
  relocations.
- **`dlclose` does not unload.**  It validates the handle but does not unmap
  objects or run `DT_FINI`/`DT_FINI_ARRAY`, matching musl's behaviour.
- **`RTLD_DEEPBIND` is not provided.**  It is not part of musl's API.
- **The loader applies every dynamic relocation type musl handles on the
  supported architectures.**  RELA and RELR tables, `COPY`, `GLOB_DAT`,
  `JUMP_SLOT`, `R_*_ADDR64`/`R_*_64`, TLS `DTPMOD`/`DTPREL`/`TPREL`, and
  TLSDESC (aarch64, x86\_64, riscv64, loongarch64) are supported;
  powerpc64le also applies `R_PPC64_UADDR64` and x86\_64 applies
  `R_X86_64_PC32`.  IFUNC (`R_*_IRELATIVE`) is not supported, matching
  musl.
- **PLT relocations are bound eagerly.**  `RTLD_LAZY` behaves like
  `RTLD_NOW`; both are valid ABI behaviour, but the loader does not defer
  `JUMP_SLOT` resolution.

## Regular expressions

- **The regex engine implements POSIX BRE/ERE matching and capture groups,
  but not backreferences, interval expressions (`{m,n}`) or
  collating/equivalence classes.**  `REG_NEWLINE` is accepted but does not
  change `^`/`$` or `.` handling.  Braces are matched literally, so a pattern
  such as `^[0-9]{4}$` does not mean "exactly four digits".  Patterns outside
  this subset may compile differently from musl.
- **Unanchored matches use leftmost-first rather than leftmost-longest
  selection.**  For example `(a|ab)*` against `"ab"` returns the `[0,1]`
  match instead of musl's `[0,2]`; anchored forms (`^(a|ab)*$`) and the
  common cases agree.  The engine is a backtracking VM with an empty-loop
  guard, not musl's parallel TNFA.

## Math

- **x86\_64 `long double` is 64-bit.**  keel does not use the 80-bit x87
  format.  The x86\_64 long-double macros are the only allowlisted
  macro-parity differences, and this is why some upstream long-double test
  vectors are skipped.
- **Some f32/f64 math functions delegate to the `libm` crate.**  This is
  deliberate where libm matches musl's fenv and last-bit behavior.  `log*`,
  `pow*`, `sin*`/`cos*`/`tan*`, `lgamma*`, `tgamma*`, `y0`, `sinh*`,
  `exp`/`expf` and `round*` keep local ports because libm changes observable
  flags or results there.
- **128-bit long-double math computes through `f64`.**  aarch64, riscv64
  and loongarch64 use IEEE binary128 and powerpc64le uses IBM double-double;
  the function signatures match those ABIs, but `math_long_double.rs`,
  `strtold` and `printf`'s `%Lf` convert through `f64`, so results lose
  precision and range beyond double.
- **powerpc64le follows the IBM double-double long-double ABI.**  This is
  the historical ppc64le default and what musl toolchains built with GCC 14
  or older use.  GCC configured with `--with-long-double-format=ieee`
  (Ubuntu 26.04's cross compiler) must compile against keel with
  `-mabi=ibmlongdouble`; `scripts/cross-smoke.sh` does this for its
  fixtures.  Mixing IEEE and IBM long-double objects is not supported.
- **loongarch64 `printf("%Lf")` reads variadic long doubles as one `f64`.**
  The value and alignment of a 16-byte variadic long double in the
  loongarch64 va_list are not decoded yet, so multiple or 128-bit long
  double arguments can print wrong values.  The other targets use the
  correct 16-byte vararg convention.
- **`vfork` forwards to `fork` and `posix_spawn` uses `fork`.**  The
  semantics are the portable ones; the copy-on-write/`vfork` performance
  optimisation is not implemented.
