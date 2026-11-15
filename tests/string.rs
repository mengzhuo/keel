// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn string_functions_under_libc_so() {
    let output = common::run_fixture("string", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "string ok\n");
}
