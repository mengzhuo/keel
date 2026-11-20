#!/bin/sh
set -e
export PATH=/usr/bin:/bin
sqlite3 --version
rm -f /tmp/test.db /tmp/test.db-wal /tmp/test.db-shm
sqlite3 /tmp/test.db "create table t(a integer, b text);
  insert into t values(1,'x'),(2,'y');
  select sum(a), group_concat(b) from t;"
sqlite3 /tmp/test.db 'pragma journal_mode=wal;
  create table w(x); insert into w values(42); select x from w;'
sqlite3 /tmp/test.db 'select count(*) from t;'
sqlite3 :memory: 'select sqlite_version(), 1+1;'
echo SQLITE_OK
