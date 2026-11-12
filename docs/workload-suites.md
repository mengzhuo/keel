# Upstream workload test suites

[`docs/workloads.md`](workloads.md) answers whether a handful of real programs
start and perform their basic task.  This page goes one step further: it runs
the programs' **own test suites** against two identical Alpine chroots, one
with stock musl 1.2.6 and one with crabc's `libldso.so` + `libc.so`.

The suites are deliberately kept separate from the release gate.  They take
several minutes, need Docker/chroot, and some upstream tests are sensitive to
the host kernel and network sandbox.  Run them manually or from the scheduled
`Workload suites` workflow; a crabc-only failure is still a regression.

## Suites

| Suite | Source | Scope | Result (2026-11-12) |
|---|---|---|---|
| CPython | Alpine `python3-tests` 3.12.16 | 40 modules, 13,737 tests | 58 modules OK; TZ differences only |
| Git | `git/t` 2.52.1, installed `git` 2.52.0 | 7 foundational t/ scripts, 599 tests | 6 scripts pass under both; one racy test in t0000 |
| nginx | `nginx/nginx-tests` `bb4d348` | 8 HTTP tests | 8/8 under both |

The Python list covers the interfaces crabc implements directly: `os`,
`io`, `fcntl`, `pwd`/`grp`, sockets and `select`, threads, subprocesses,
signals, SSL, hashing, `ctypes`, `sqlite3`, asyncio, time/math/locale,
importlib, multiprocessing, logging, datetime, codecs and archives.  The Git
list exercises repository setup, CRLF handling, option parsing, pathspecs,
the index, diff formatting and error paths.  The nginx list covers static
files, autoindex, rewrites, headers, gzip, maps, SSI and request limiting.

## Running

```bash
sudo ./scripts/workload-suites.sh              # all three suites
sudo ./scripts/workload-suites.sh python       # one suite
sudo ./scripts/workload-suites.sh git nginx
```

The script builds `libc.so`/`libldso.so` in release mode, creates the two
rootfs copies with `apk` (Docker or podman), bind-mounts `/proc`, `/sys`,
`/dev` and the shared work directory, builds Git's `t/helper/test-tool` under
crabc, runs both sides, and writes logs plus normalized failure lists to
`workload-suites/reports/`.

Useful environment variables:

- `ALPINE_MIRROR` — default `https://dl-cdn.alpinelinux.org`; for a slow
  mirror use e.g. `https://mirrors.aliyun.com`.
- `SUITE_DIR` — work/report directory (default `workload-suites/`).
- `GIT_VERSION` — Git source version (default `2.52.1`).
- `NGINX_TESTS_REF` — pinned nginx-tests commit.
- `STRICT=1` — also fail on the documented known differences.
- `DOCKER=podman` — use podman instead of docker.

`STRICT=1` is useful when checking whether a known gap has been fixed.

## Results

The latest run used crabc 0.7.8 plus the loader/TLS/regex fixes below.

### CPython

```text
musl:  59 modules OK,  9 failed, 13,737 tests run
crabc: 58 modules OK, 10 failed, 13,737 tests run
```

The baseline failures are environment-related and occur with stock musl too:
`test_asyncio.test_events`, `test_ctypes`, `test_locale`,
`test_multiprocessing`, `test_os`, and occasionally `test_re`.

The only crabc-only difference is time-zone support:

- `test_time` — `tzset()` is a stub and `localtime()` is `gmtime()`, so
  `TZ=EST5EDT` does not change the result.
- `test_datetime` — the same gap in `astimezone()` and naive
  `datetime.timestamp()`.
- `test_logging` (not always reached) — DST-aware log rotation depends on
  the same local-time conversion.

This is a real feature gap, not an ABI mismatch: crabc currently has no
`/usr/share/zoneinfo` reader and no POSIX `TZ` rule parser.  It is tracked
here rather than hidden by excluding the modules.

`test_ssl` is a timing-sensitive upstream test.  The TLS 1.3 wrong-certificate
case can see `ECONNRESET` instead of the expected alert when the whole module
runs; the single test usually passes on its own.  It is listed as a known
difference so it does not hide other SSL regressions.

### Git

```text
musl:  6 passed, 1 failed
crabc: 6 passed, 1 failed
```

`t0000-basic.sh` test 82 (`no diff after checkout and update-index --refresh`)
fails identically in both chroots because of racy index timestamps on the
shared filesystem.  The remaining six scripts pass under both runtimes.  A
crabc-only failure would be reported by the harness.

### nginx

```text
musl:  8 passed, 0 failed
crabc: 8 passed, 0 failed
```

The tests run nginx as root (`TEST_NGINX_GLOBALS='user root;'`) so the
worker can read the test directory, and load the packaged dynamic modules
from `/usr/lib/nginx/modules`.

## Bugs this harness found

Each of these was reproduced by a suite and fixed on `main` after 0.7.8:

- `gethostbyaddr()` ignored `/etc/hosts`; it now goes through
  `getnameinfo()` like musl.
- `getaddrinfo()` did not parse RFC 4007 `%ifname`/`%scope` suffixes.
- `pthread_sigmask()` returned `-1`/errno instead of the POSIX error number;
  Python set `errno = -1` and `strerror()` panicked on the negative value.
- `pselect()` passed the sigset pointer straight to `pselect6`, which expects
  `{ sigmask, sigsetsize }`.  GNU make blocks `SIGCHLD` and relies on
  `pselect` to unblock it, so `make -j` deadlocked after the first batch of
  jobs.
- `regoff_t` was `int`, while musl uses `long`.  Prebuilt BusyBox binaries
  therefore saw a different `regmatch_t` layout; `sed` duplicated output and
  `awk` ran out of memory.
- The regex engine did not support `.`, compiled `+` as a possibly-empty
  match, and sized its backtracking stack from the bytecode instead of the
  input.  This broke BusyBox `expr`, `sed -g` and Git's test helpers.

These are exactly the class of bug a small smoke workload misses: they need
the program's own tests, or at least its real command-line tools, to reach
the code path.

## Not covered

- Full CPython, Git and nginx suites: the scripts run a curated, comparable
  subset.  Extend the arrays at the top of `scripts/workload-suites.sh` when
  a specific area needs more coverage.
- Network-dependent tests (`-u network` is disabled), TLS client
  certificates, IPv6-only nginx tests, and the PostgreSQL/Redis/SQLite
  upstream suites.  The latter need a source build and are candidates for a
  follow-up.
- Release gating: these suites are intentionally not a `needs:` dependency
  of `publish`.  The release gate remains `test` + `macro-parity`; workload
  suites run on demand and weekly.
