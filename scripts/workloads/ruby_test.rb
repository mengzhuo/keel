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
