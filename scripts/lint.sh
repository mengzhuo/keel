#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause

# Rust lints used by CI and local development.
#
# The loader, utilities and header checker are normal Rust code and are held
# to the full clippy warning set.  keel-libc is a literal musl port: clippy's
# style/complexity groups fire on intentional C-style expressions and on the
# spelled-out floating-point constants in the math code.  It is therefore
# checked with the style groups allowed, while correctness and rustc warnings
# remain fatal.  The four allowed correctness lints are deliberate:
#
#   eq_op                       - x - x / x / x used to raise the expected
#                                 floating-point exceptions in math helpers
#   approx_constant             - musl spells out PI and 2/PI bit patterns
#   absurd_extreme_comparisons  - hand-ported range checks around type limits
#   never_loop                  - ported retry loops with a guaranteed exit
set -euo pipefail

cd "$(dirname "$0")/.."

cargo clippy -p keel-rt -p keel-utils -p keel-header-check \
    --all-targets -- -D warnings

cargo clippy -p keel-libc --all-targets -- \
    -D warnings \
    -A clippy::all \
    -D clippy::correctness \
    -A clippy::eq_op \
    -A clippy::approx_constant \
    -A clippy::absurd_extreme_comparisons \
    -A clippy::never_loop
