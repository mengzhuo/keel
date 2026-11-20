#!/bin/sh
set -e
export PATH=/usr/bin:/bin
rm -f /tmp/keel.rdb
redis-server --version
redis-cli --version
redis-server --port 16379 --daemonize yes --dir /tmp \
  --dbfilename keel.rdb --save "" --appendonly no
sleep 0.5
redis-cli -p 16379 ping
redis-cli -p 16379 set k hello
redis-cli -p 16379 get k
redis-cli -p 16379 incr counter
redis-cli -p 16379 rpush list a b c
redis-cli -p 16379 lrange list 0 -1
redis-cli -p 16379 hset h f v
redis-cli -p 16379 hget h f
redis-cli -p 16379 save
redis-cli -p 16379 dbsize
redis-cli -p 16379 shutdown nosave
echo REDIS_OK
