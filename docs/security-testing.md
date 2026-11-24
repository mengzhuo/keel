# Security testing

keel's test suite (see `AGENTS.md`) checks compatibility against musl; this
document covers the additional robustness work directed at untrusted input.
All three tools run on a developer machine with `musl-gcc`, `gcc` and a built
`target/debug/libc.so`.

## Deterministic parser fuzz

`tests/fixtures/fuzz_parsers_test.c` (driver: `tests/fuzz_parsers.rs`) feeds a
fixed pseudo-random corpus through the parsers that consume untrusted data:

- `inet_aton`/`inet_pton`/`inet_ntop`
- `strtod`/`strtol`/`strtoul`
- `fnmatch` with each flag
- `regcomp`/`regexec`
- DNS name decompression (`dn_expand`)
- `getmntent` over `fmemopen` buffers
- `iconv` byte streams and `iconv(cd, NULL, ...)` state resets
- `strxfrm`/`strptime`
- `glob`, `getopt`, `fgetpwent`/`fgetgrent`/`fgetspent`
- `TZ` strings and `localtime_r`
- `snprintf` with randomized format skeletons

It is deterministic (xorshift with a fixed seed), so a regression is
reproducible and CI fails on a crash or hang. It is not coverage-guided; it
exists to keep the corpus and the crash-free property in the normal test
matrix.

```sh
cargo test --test fuzz_parsers
```

## Differential fuzz against musl

`scripts/diff-fuzz.c` is compiled twice, once against `target/debug/libc.so`
and once statically against `musl-gcc`, and the same deterministic corpus is
fed to both. The oracle is **musl**, not glibc: glibc differs on deliberately
strict `inet_aton` parsing and on `fnmatch` patterns with a trailing backslash
or an unmatched `[`. Any stdout, exit-status or crash difference is reported.

```sh
./scripts/diff-fuzz.sh 5000
```

Covered: `strtod`, `strtol`, `strtoul`, `inet_aton`, `inet_pton`,
`fnmatch`, `strxfrm`, `strptime`, `snprintf`, and `regcomp`/`regexec` over a
set of patterns with quantifiers and alternation.

## ELF loader mutation fuzz

`scripts/elf-fuzz.sh` builds a tiny valid DSO, flips bits outside its
executable segments, and asks `dlopen(..., RTLD_NOW)` in a subprocess to load
each mutant. A signal or `abort()` is a loader-robustness failure: malformed
input must be rejected, not crash the process. Executable bytes are excluded
because executing corrupted code says nothing about the parser.

```sh
./scripts/elf-fuzz.sh 500 3     # iterations, seed
```

The loader hardening driven by this fuzzer:

- segment `p_filesz <= p_memsz`, no file range past EOF, page congruence;
- checked PT_LOAD arithmetic and a bounded total image size;
- TLS range validation;
- `PT_DYNAMIC` and every dynamic table (`DT_SYMTAB`, `DT_STRTAB`,
  `DT_GNU_HASH`, `DT_HASH`, `DT_VERSYM`, `DT_INIT`, `DT_INIT_ARRAY`, RELA/
  JMPREL/RELR) must lie inside a mapped PT_LOAD, not merely inside the
  image's min/max span (holes between segments are common in malformed input);
- relocation slots must be inside a writable mapped range; RELR slots are
  validated through `apply_relr_with`;
- constructor pointers must resolve into an executable mapped segment.

The last crashing mutant is saved to `/tmp/opencode/elf-crash.so` for triage
with `gdb` against the `dlopen` harness.

## Guard-page bounds testing

`tests/fixtures/string_test.c` (driver `tests/string.rs`) and
`tests/fixtures/compat_test.c` (`tests/compat.rs`) map a writable region whose
last page is `PROT_NONE`, then place the source, destination or input buffer
so it ends exactly at that page. A primitive that reads or writes even one
byte past the requested range faults instead of silently passing.

The string fixture covers `memcpy`, `memmove`, `memset`, `memcmp`, `memchr`,
`memrchr`, `strlen`, `strnlen`, `strchrnul`, `strchr`, `strrchr`, `strcmp`,
`strncmp`, `strstr`, `memmem`, `strcasestr`, `strcspn`, `strspn`, `strpbrk`,
`strcpy`, `strncpy`, `strcat` and `strncat`, with lengths from 1 to 4096 and
the operand at either edge. The compat fixture extends this to wide strings,
`strtok`, `strcasecmp`/`strverscmp`, `strlcpy`/`memccpy`/`strxfrm`,
`sscanf`/`strtol`/`strtod`/`strptime`/`crypt`, `mbrtowc`/`wcrtomb`, `iconv`,
`fmemopen`+`getline`, `fnmatch`/`regcomp`/`regexec`, and format strings that
end with `%`.

```sh
cargo test --test string
cargo test --test compat
```

This found two real bugs. `memchr` used unaligned 16/32-byte loads and read
into the next page when the range ended on a page boundary; BusyBox `tar`
crashed in `strnlen` on such a buffer. It now uses aligned loads with
first/last-block masks, so it never crosses a page and never reports a match
outside `[s, s+n)`; it may still load bytes after `n` within the same page and
mask them out. `strnlen(s, SIZE_MAX)` wrapped `start + n` and returned
`SIZE_MAX`; the end is now a `checked_add`. The non-`mem*`/`str*` families
passed with no bounds findings; the only observed difference was the C-locale
UTF-8 behaviour recorded in `docs/limitations.md`.

## Supply chain

- `deny.toml` + `scripts/audit-deps.sh` (cargo-deny/cargo-audit)
- Dependabot for Cargo and GitHub Actions
- Tests and the workload matrix run in CI; release tagging requires a green
  `CI status` check (see `SECURITY.md` and `docs/packages.md`)
