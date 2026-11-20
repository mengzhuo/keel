#!/bin/sh
set -e
export PATH=/usr/bin:/bin
php83 -v | head -1
php83 /work/php_test.php
echo PHP_OK
