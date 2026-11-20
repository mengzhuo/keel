# Bash

**Status: ✅ passes under keel**

| | |
|---|---|
| Package | `bash-5.3.3-r1` (Alpine 3.23) |
| Version | `GNU bash, version 5.3.3(1)-release (x86_64-alpine-linux-musl)` |
| Baseline | same package with stock musl 1.2.6 |

## What was tested

BusyBox `sh` covers the basic shell surface; GNU Bash adds parts of libc that
are easy to miss: `readline`, job control, process substitution (`/dev/fd`),
`coproc` pipes, `mapfile`, arrays, POSIX regex captures (`BASH_REMATCH`), and
the EXIT trap.

- `BASH_VERSION`;
- indexed and associative arrays;
- `[[ =~ ]]` with a capture group;
- `diff <(...) <(...)` process substitution;
- `mapfile` and a pipeline;
- function calls and command substitution;
- `set -m`, a background `sleep`, `kill -0` and `wait`;
- `coproc` and its pipes;
- an `EXIT` trap.

## Commands

Run the script with Bash itself (not `/bin/sh`):

```sh
/bin/bash /work/bash.sh
```

```bash
#!/bin/bash
set -euo pipefail
echo "BASH_VERSION=$BASH_VERSION"
a=(one two three)
echo "array ${#a[@]} ${a[1]}"
declare -A h=([x]=1 [y]=2)
echo "assoc ${h[x]}${h[y]}"
if [[ abc123 =~ ^[a-z]+([0-9]+)$ ]]; then
  echo "regex ${BASH_REMATCH[1]}"
fi
diff <(printf 'a\n') <(printf 'a\n') && echo "psub ok"
mapfile -t lines < <(printf 'x\ny\n')
echo "mapfile ${lines[0]}${lines[1]}"
printf 'one\ntwo\n' | while read -r l; do echo "pipe $l"; done
f() { echo "func $1"; }
f hello
echo "cmdsub $(printf 'sub')"
{ echo block; } > /tmp/bash.out
cat /tmp/bash.out
set -m
sleep 0.2 &
p=$!
kill -0 "$p" && echo "bg ok"
wait "$p"
echo "wait $?"
coproc { echo coproc-ok; }
read -r line <&"${COPROC[0]}"
echo "coproc $line"
trap 'echo trap-exit' EXIT
echo BASH_OK
```

## Output under keel

```text
BASH_VERSION=5.3.3(1)-release
array 3 two
assoc 12
regex 123
psub ok
mapfile xy
pipe one
pipe two
func hello
cmdsub sub
block
bg ok
wait 0
coproc coproc-ok
BASH_OK
trap-exit
```

The `trap-exit` line follows `BASH_OK` because the EXIT trap runs last.

## Baseline comparison

Byte-for-byte identical output and exit status under musl.

## Notes

- This workload exposed a regex bug: `regcomp` left `re_nsub` at zero and
  `regexec` wrote only `pmatch[0]`, so `BASH_REMATCH` had no capture groups.
  The regex VM now tracks group boundaries and reports `re_nsub`, and
  `tests/fixtures/regex_test.c` covers nested, optional and repeated groups.
- Job control is exercised without a controlling terminal. Interactive
  readline editing is not part of this script.
