<?php
printf("core %s %d %s %s\n", PHP_VERSION, PHP_INT_SIZE, json_encode(['a' => 1]), substr(hash('sha256', 'x'), 0, 8));
$exts = ['openssl', 'sqlite3', 'curl', 'mbstring', 'iconv', 'sockets', 'pcntl', 'posix'];
$loaded = array_map(fn($e) => extension_loaded($e) ? $e : "-", $exts);
printf("ext %s\n", implode(' ', $loaded));
$db = new SQLite3(':memory:');
$db->exec('create table t(x)');
$db->exec('insert into t values(7)');
printf("sqlite %s %s\n", SQLite3::version()['versionString'], $db->querySingle('select x from t'));
$ct = openssl_encrypt('hello', 'aes-256-cbc', str_repeat('k', 32), OPENSSL_RAW_DATA, str_repeat('i', 16));
printf("openssl %s %s\n", OPENSSL_VERSION_TEXT, substr(hash('sha256', $ct), 0, 8));
printf("mb %s %d\n", mb_strtoupper('hello'), strlen(iconv('UTF-8', 'ISO-8859-1', 'héllo')));
$n = preg_match('/^\p{L}+([0-9]+)$/u', 'abc123', $m);
printf("pcre %d %s\n", $n, $m[1]);
$ch = curl_init('http://127.0.0.1:18080/');
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$body = curl_exec($ch);
printf("curl %s %s\n", curl_getinfo($ch, CURLINFO_HTTP_CODE), trim($body));
curl_close($ch);
$pid = pcntl_fork();
if ($pid === 0) {
    exit(7);
}
pcntl_waitpid($pid, $status);
printf("pcntl %d %d\n", pcntl_wexitstatus($status), posix_getpid() > 0);
$f = tmpfile();
fwrite($f, "tmp\n");
rewind($f);
printf("stream %s\n", trim(fgets($f)));
fclose($f);
printf("random %d\n", strlen(random_bytes(8)));
$hash = password_hash('pw', PASSWORD_ARGON2ID);
printf("argon2 %s\n", password_verify('pw', $hash) ? 'ok' : 'bad');
$p = proc_open('printf proc-ok', [1 => ['pipe', 'w']], $pipes);
printf("proc %s\n", trim(stream_get_contents($pipes[1])));
fclose($pipes[1]);
proc_close($p);
echo "PHP_INNER_OK\n";
