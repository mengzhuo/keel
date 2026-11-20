#!/bin/sh
set -e
export PATH=/usr/bin:/bin
ruby -v
ruby /work/ruby_test.rb
echo RUBY_OK
