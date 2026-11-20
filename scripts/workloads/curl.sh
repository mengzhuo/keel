#!/bin/sh
set -e
export PATH=/usr/bin:/bin
curl --version | head -1
curl -sS file:///etc/passwd | head -1
curl -sS http://127.0.0.1:18080/
curl -sS -o /dev/null -w 'http_code=%{http_code}\n' http://127.0.0.1:18080/
curl -sS -o /dev/null -w 'https_code=%{http_code}\n' https://example.com/
echo CURL_OK
