// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn string_fast_paths_under_libc_so() {
    let output = common::run_fixture("string_fast", &["-O2"]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "string fast ok\n");
}
