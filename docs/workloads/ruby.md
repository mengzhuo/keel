# Ruby

**Status: ✅ passes under keel**

| | |
|---|---|
| Package | `ruby-3.4.12-r0` (Alpine 3.23) |
| Version | `ruby 3.4.12 (2026-10-23 revision 366f5fdc00) +PRISM [x86_64-linux-musl]` |
| Baseline | same package with stock musl 1.2.6 |

## What was tested

Ruby links against `libruby`, `libgmp`, `libucontext` and `libz`, and its
standard library loads native extensions for OpenSSL, zlib, Psych/YAML and
sockets. Fibers use `libucontext`, which calls
`getcontext`/`makecontext`/`swapcontext` from libc.

- version and platform;
- `Integer.sqrt`, UTF-8 strings, `JSON`;
- OpenSSL SHA-256 and AES-256-CBC;
- zlib deflate and CRC32;
- YAML dump/load (Psych + libyaml);
- four Ruby threads and `Thread#value`;
- fibers (`Fiber.yield`/`resume`);
- `fork`/`Process.wait2`, backticks (`popen`), `Tempfile`;
- a TCP socket against the local HTTP server;
- `Random.bytes`, regexp capture, `Etc.getpwuid` and `SecureRandom`.

## Commands

```sh
export PATH=/usr/bin:/bin
ruby -v
ruby /work/ruby_test.rb
echo RUBY_OK
```

The driver script:

```ruby
require 'json'
require 'openssl'
require 'zlib'
require 'yaml'
require 'tempfile'
require 'socket'
require 'etc'
require 'securerandom'

puts "version #{RUBY_VERSION} #{RUBY_PLATFORM}"
puts "core #{1 + 1} #{Integer.sqrt(2**64)} #{('x' * 3).encoding}"
puts "json #{JSON.generate({a: 1})}"
puts "openssl #{OpenSSL::OPENSSL_VERSION} #{OpenSSL::Digest::SHA256.hexdigest('x')[0, 8]}"
cipher = OpenSSL::Cipher.new('aes-256-cbc').encrypt
cipher.key = 'k' * 32
cipher.iv = 'i' * 16
ct = cipher.update('hello') + cipher.final
puts "cipher #{OpenSSL::Digest::SHA256.hexdigest(ct)[0, 8]}"
puts "zlib #{Zlib::Deflate.deflate('x' * 100).bytesize} #{Zlib.crc32('x')}"
puts "yaml #{YAML.load(YAML.dump({'a' => 1}))['a']}"
threads = 4.times.map { |i| Thread.new { i * 10 } }
puts "threads #{threads.map(&:value).sum}"
f = Fiber.new { Fiber.yield 1; 2 }
puts "fiber #{f.resume} #{f.resume}"
pid = fork { exit 3 }
_, status = Process.wait2(pid)
puts "fork #{status.exitstatus}"
puts "popen #{`printf popen-ok`}"
Tempfile.create('keel') do |t|
  t.write("tmp\n")
  t.rewind
  puts "tempfile #{t.read.strip}"
end
s = TCPSocket.new('127.0.0.1', 18080)
s.write("GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
resp = s.read
body = resp.split("\r\n\r\n", 2)[1]
puts "socket #{resp.lines.first.strip} #{body.to_s.strip}"
s.close
puts "random #{Random.bytes(8).bytesize}"
puts "regex #{'abc123'[/^[a-z]+(\d+)$/, 1]}"
puts "etc #{Etc.getpwuid(Process.uid).name}"
puts "secure #{SecureRandom.hex(4).length}"
puts "RUBY_INNER_OK"
```

## Output under keel

```text
ruby 3.4.12 (2026-10-23 revision 366f5fdc00) +PRISM [x86_64-linux-musl]
version 3.4.12 x86_64-linux-musl
core 2 4294967296 UTF-8
json {"a":1}
openssl OpenSSL 3.5.9 29 Sep 2026 2d711642
cipher 33187cb9
zlib 12 2363233923
yaml 1
threads 60
fiber 1 2
fork 3
popen popen-ok
tempfile tmp
socket HTTP/1.0 200 OK hello from keel workload test
random 8
regex 123
etc root
secure 8
RUBY_INNER_OK
RUBY_OK
```

## Baseline comparison

Identical output and exit status under musl.

## Notes

- Fibers exercise `libucontext`'s context-switch path. Ruby also creates
  threads before the first fiber, so this covers a mix of pthread TLS,
  `swapcontext` and native extension constructors.
- `SecureRandom.hex(4).length` and `Random.bytes(8).bytesize` print only
  lengths, so the output stays deterministic.
- Third-party native gems were not installed.
