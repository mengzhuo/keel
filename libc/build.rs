// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

fn main() {
    // Bind internal function calls, but leave public data symbols
    // preemptible so executables can use COPY relocations (getopt's optind,
    // stdio's stdin, ...).  The interpreter self-relocates the remaining
    // RELA table in _start.
    println!("cargo:rustc-cdylib-link-arg=-Wl,-Bsymbolic-functions");
}
