#!/bin/sh
set -e
export PATH=/usr/sbin:/usr/bin:/bin
mkdir -p /tmp/logs
nginx -v
nginx -t -c /work/nginx.conf -p /tmp
rm -f /tmp/nginx.pid /tmp/nginx-error.log
nginx -c /work/nginx.conf -p /tmp &
P=$!
sleep 1
curl -sS http://127.0.0.1:18081/
curl -sS http://127.0.0.1:18081/api
curl -sS -o /dev/null -w 'code=%{http_code}\n' http://127.0.0.1:18081/index.html
kill -QUIT $P
wait $P
echo NGINX_OK
