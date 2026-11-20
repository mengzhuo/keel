# PHP

**Status: ✅ passes under keel**

| | |
|---|---|
| Package | `php83-8.3.36-r0` plus the extensions listed below (Alpine 3.23) |
| Version | `PHP 8.3.36 (cli) (built: Oct 22 2026 08:03:19) (NTS)` |
| Baseline | same package with stock musl 1.2.6 |

## What was tested

The PHP CLI is a strong test of startup-time `dlopen`: each extension is a
separate shared object listed in `/etc/php83/conf.d/*.ini` and loaded before
the script runs. The installed extensions are `openssl`, `sqlite3`,
`pdo_sqlite`, `mbstring`, `iconv`, `curl`, `phar`, `posix`, `pcntl` and
`sockets`.

- `php83 -v` and extension loading through the `conf.d` scan directory;
- JSON, SHA-256 and integer sizes;
- in-memory SQLite3 (`CREATE`, `INSERT`, `SELECT`);
- OpenSSL AES-256-CBC (EVP) and `OPENSSL_VERSION_TEXT`;
- `mbstring` and `iconv`;
- PCRE2 with a Unicode property and a capture group;
- libcurl HTTP GET against the local server;
- `pcntl_fork` / `pcntl_waitpid` and `posix_getpid`;
- `tmpfile()` streams and `proc_open` pipes;
- `random_bytes` (`getrandom`) and Argon2id `password_hash`/`password_verify`.

## Commands

```sh
export PATH=/usr/bin:/bin
php83 -v | head -1
php83 /work/php_test.php
echo PHP_OK
```

The driver script:

```php
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
```

## Output under keel

```text
PHP 8.3.36 (cli) (built: Oct 22 2026 08:03:19) (NTS)
core 8.3.36 8 {"a":1} 2d711642
ext openssl sqlite3 curl mbstring iconv sockets pcntl posix
sqlite 3.53.4 7
openssl OpenSSL 3.5.9 29 Sep 2026 33187cb9
mb HELLO 5
pcre 1 123
curl 200 hello from keel workload test
pcntl 7 1
stream tmp
random 8
argon2 ok
proc proc-ok
PHP_INNER_OK
PHP_OK
```

## Baseline comparison

Identical output and exit status under musl.

## Notes

- The first run loaded none of the extensions under keel. The cause was
  `scandir()` returning an array of `char *` name strings instead of
  `struct dirent *` records; PHP's ini scanner read garbage `d_name` values
  and skipped every `.ini` file. `scandir` now copies whole dirent records,
  and `alphasort`/`versionsort` read `d_name` from them.
  `tests/fixtures/scandir_test.c` covers the layout, the selector callback and
  both sort functions.
- `php83 -m` is a quick way to verify that startup `dlopen` of all extensions
  succeeded.
