// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn ctype_functions_under_libc_so() {
    let output = common::run_fixture("ctype", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "ctype ok\n");
}
