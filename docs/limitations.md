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

## Threading

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
- **No Unicode width tables.**  `wcwidth` returns one column for every
  printable character and `wcswidth` counts every non-NUL character as one
  column.  Combining marks and double-width characters are therefore wrong.
- **`iconv` supports a fixed encoding set.**  UTF-8/16/32, ASCII, the
  ISO-8859 family, Windows-1251/1252, KOI8-R, GBK/GB2312, Big5, EUC-JP and
  Shift-JIS are implemented; other musl charmaps are not available.

## Process and file APIs

- **`at_quick_exit` callbacks are not run.**  The function accepts the
  callback but `quick_exit` is an alias for `_exit`.
- **`getpass` leaves terminal echo enabled.**  It reads from stdin without
  the termios echo handling that musl uses.
- **The base page size is hardcoded to 4096.**  `getpagesize` and
  `sysconf(_SC_PAGE_SIZE)` do not query `AT_PAGESZ`, so they are wrong on
  aarch64/riscv64 kernels configured with 16 KiB or 64 KiB pages.
- **`fnmatch` rejects long components.**  Path components longer than 1024
  bytes return `FNM_NOMATCH` instead of being matched with a heap buffer.
- **`tsearch` is unbalanced.**  The search tree is a plain BST, so lookups
  and inserts are O(n) in the worst case; musl uses an AVL tree.

## Loader

- **The standalone `keel` mapper drops the environment.**  It builds an
  `argv` and auxiliary vector for the target but gives it an empty `envp`.
  Dynamic binaries using `libc.so` as the interpreter are not affected.
- **The executable's direct `DT_NEEDED` list is capped at 32 entries.**
  Additional direct dependencies of the main executable are ignored; the
  loader's object and dependency tables for DSOs grow dynamically.
- **`dlclose` does not unload.**  It validates the handle but does not unmap
  objects or run `DT_FINI`/`DT_FINI_ARRAY`, matching musl's behaviour.
- **`RTLD_DEEPBIND` is not provided.**  It is not part of musl's API.

## Math

- **x86\_64 `long double` is 64-bit.**  keel does not use the 80-bit x87
  format.  The x86\_64 long-double macros are the only allowlisted
  macro-parity differences, and this is why some upstream long-double test
  vectors are skipped.
- **aarch64/riscv64 long-double math computes through `f64`.**  The function
  signatures use `f128`, but `math_long_double.rs` currently converts through
  `f64`, so results can lose precision beyond double range.
- **`vfork` forwards to `fork` and `posix_spawn` uses `fork`.**  The
  semantics are the portable ones; the copy-on-write/`vfork` performance
  optimisation is not implemented.
