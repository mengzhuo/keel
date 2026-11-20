#!/bin/sh
set -e
export PATH=/usr/bin:/bin
python3 -V
python3 -c 'import sys, os, json, hashlib, re, datetime
print("core", sys.version_info[:2], json.dumps({"a": 1}),
      hashlib.sha256(b"x").hexdigest()[:8])'
python3 -c 'import ssl; print("ssl", ssl.OPENSSL_VERSION)'
python3 -c 'import sqlite3
c = sqlite3.connect(":memory:")
c.execute("create table t(x)")
c.execute("insert into t values(7)")
print("sqlite3", c.execute("select x from t").fetchone()[0])'
python3 -c 'import ctypes; print("ctypes", ctypes.CDLL(None).getpid() > 0)'
python3 -c 'import zlib, bz2, lzma
print("compress", len(zlib.compress(b"x"*100)),
      len(bz2.compress(b"x"*100)), len(lzma.compress(b"x"*100)))'
echo PY_OK
