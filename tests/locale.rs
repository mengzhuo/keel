// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn locale_functions_under_libc_so() {
    let output = common::run_fixture("locale", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "locale ok\n");
}
