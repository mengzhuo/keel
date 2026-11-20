#!/bin/sh
set -e
export PATH=/usr/bin:/bin
node -v
node -e 'console.log("core", process.version, 1+1, require("os").platform())'
node -e 'const fs = require("fs")
fs.writeFileSync("/tmp/n.txt", "hello")
console.log("fs", fs.readFileSync("/tmp/n.txt", "utf8"))'
node -e 'const http = require("http")
http.get("http://127.0.0.1:18080/", r => {
  let d = ""; r.on("data", c => d += c)
  r.on("end", () => console.log("http", d.trim()))
})'
node -e 'const zlib = require("zlib")
console.log("zlib", zlib.gzipSync("hello").length)'
node -e 'const crypto = require("crypto")
console.log("crypto",
  crypto.createHash("sha256").update("x").digest("hex").slice(0, 8))'
echo NODE_OK
