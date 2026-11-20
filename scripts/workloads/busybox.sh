#!/bin/sh
set -e
export PATH=/usr/bin:/bin
busybox | head -1
echo "applets: $(busybox --list | wc -l)"
busybox sh -c 'echo shell-ok; ls / >/dev/null && echo ls-ok'
busybox grep -c root /etc/passwd
busybox awk 'BEGIN { print "awk", 1+1 }'
busybox sed -n '1p' /etc/passwd
busybox find /etc -maxdepth 1 -name passwd
busybox wget -qO- -T 10 http://127.0.0.1:18080/
echo wget-ok
# Only print stable columns: PID 1's CPU time changes between runs.
busybox ps | awk 'NR == 1 { print $1, $2, $4 } NR > 1 && $1 == 1 { print $1, $2, $4 }'
echo BUSYBOX_OK
