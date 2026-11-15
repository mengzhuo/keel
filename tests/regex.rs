// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn regex_functions_under_libc_so() {
    let output = common::run_fixture("regex", &[]);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "regex ok\n");
}
