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
