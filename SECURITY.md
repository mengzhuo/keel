# Security policy

keel is a Rust `no_std` musl-compatible C library, dynamic linker and
standalone loader.  It runs unmodified musl-linked binaries, so its attack
surface includes every parser and system-call wrapper in the C library plus
the ELF loader itself.

## Supported versions

Only the latest `0.1.x` release and the `main` branch receive security fixes.
There is no long-term-support branch yet.  The project is experimental
(`0.1.x`); see `docs/limitations.md` before relying on it in a privileged
environment.

## Reporting a vulnerability

Please report suspected vulnerabilities **privately** through GitHub Security
Advisories:

<https://github.com/mengzhuo/keel/security/advisories/new>

Do not open a public issue for an unfixed vulnerability.  Include:

- the affected component and version/commit;
- a minimal reproducer (source, input, or commands);
- the impact you believe it has (crash, information leak, privilege
  escalation, sandbox escape, ...);
- whether the issue is already public.

We aim to acknowledge within 7 days, provide an initial assessment within 14
days, and coordinate disclosure within 90 days.  There is no bug bounty.
Credit is given in the advisory and release notes unless you ask otherwise.

## Trust model and scope

- **Running an untrusted executable is code execution.**  The standalone
  `keel` loader and the dynamic linker are not sandboxes.  A malicious ELF is
  treated as untrusted input only for parser-robustness purposes (a crash or
  out-of-bounds access in the loader is still a bug, but it is not a
  privilege boundary by itself).
- **Setuid/setgid binaries.**  In `AT_SECURE` mode the loader ignores
  `LD_LIBRARY_PATH` and refuses to expand the main executable's `$ORIGIN`;
  this matches musl.  The C library itself does not decide which environment
  variables an application may trust: use `secure_getenv()` for that.
- **The C library is not a sandbox.**  `wordexp`, `system`, `popen`, and
  `dlopen` do what their APIs say; applications must use `WRDE_NOCMD`,
  absolute paths, and their own privilege separation where required.
- **Dependencies.**  `keel-libc` intentionally depends only on `libm`.
  `keel-rt` and `keel-utils` use the Rust standard library.  New runtime
  dependencies require a security review.

## Hardening in the current design

- `PT_GNU_RELRO` is applied after relocations (main executable, interpreter
  and `dlopen`ed objects).
- The stack canary is initialized from `AT_RANDOM` in every TCB, including
  threads created before/after `dlopen`.
- The loader applies relocations eagerly; there is no writable lazy-binding
  PLT.
- `LD_PRELOAD` is not supported.  `LD_LIBRARY_PATH` is ignored in secure mode.
- `malloc` uses checked size arithmetic; `mkstemp`/`tmpfile`/`sem_open`
  temporary names use `getrandom` with `O_EXCL`.
- Parsers that consume untrusted data (DNS names, TZif, glob/regex, printf/
  scanf, iconv, mntent/ethers, netlink) have explicit length/depth limits.
- `cargo test --workspace`, the libc-test harness, the macro/inline parity
  gates, and the Alpine workload matrix run in CI; see `.github/workflows/`.
- Robustness tooling: a deterministic parser-fuzz fixture
  (`cargo test --test fuzz_parsers`), differential fuzzing against musl
  (`scripts/diff-fuzz.sh`) and ELF-loader mutation fuzzing
  (`scripts/elf-fuzz.sh`).  See `docs/security-testing.md`.

## Release verification

Release artifacts are published on GitHub with `SHA256SUMS`:

```sh
sha256sum -c SHA256SUMS
```

APK packages are signed only when the release environment provides the
`APK_SIGN_KEY_B64` secret (an apk-tools RSA private key, base64-encoded).
If a release is unsigned, `apk` requires `--allow-untrusted`; in that case
verify `SHA256SUMS` against the GitHub release before installing.  The
packages contain the prebuilt `libc.so`, headers, CRT objects and utilities;
no post-install build step is performed.

## Dependency and supply-chain checks

- `deny.toml` configures `cargo-deny` (advisories, licenses, sources, bans).
- `scripts/audit-deps.sh` runs `cargo-deny`/`cargo-audit` when installed.
- Dependabot is configured for Cargo and GitHub Actions updates.
- Releases require a successful `CI status` check for the tagged commit and
  publish through crates.io Trusted Publishing (OIDC).

## Known limitations

`docs/limitations.md` is the authoritative list of intentional differences
and unsupported features (stdio locking, locale data, `printf` positional
arguments, `{m,n}` regex intervals, detached-thread TLS retention, and so
on).  A limitation is not automatically a vulnerability, but it can become
one when an application relies on the missing behavior for validation.
