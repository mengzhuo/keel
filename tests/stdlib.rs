// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn stdlib_functions_under_libc_so() {
    let output = common::run_fixture("stdlib", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "stdlib ok\n");
}
