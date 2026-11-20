#!/bin/sh
set -e
export PATH=/usr/libexec/postgresql18:/usr/bin:/bin
export PGDATA=/var/lib/postgresql/data
export PGHOST=/tmp PGPORT=15432 PGUSER=postgres
rm -rf "$PGDATA"
mkdir -p "$PGDATA"
initdb -D "$PGDATA" --no-locale -U postgres
echo INITDB_OK
pg_ctl -D "$PGDATA" -o "-p 15432 -k /tmp -c listen_addresses=''" \
       -l /tmp/pg.log -w start
echo "pg_ctl start rc=$?"
psql -Atc 'select version();'
psql -Atc 'select 1+1;'
psql -c 'create table t(a int, b text);'
psql -c "insert into t values (1,'alpha'),(2,'beta'),(3,'gamma');"
psql -Atc 'select sum(a), string_agg(b, chr(44) order by a) from t;'
psql -Atc 'create index ti on t(a); select count(*) from t where a > 1;'
psql -Atc 'select pg_database_size(current_database()) > 0;'
pg_ctl -D "$PGDATA" -m fast -w stop
echo "pg_ctl stop rc=$?"
echo PG_OK
