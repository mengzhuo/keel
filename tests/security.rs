// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn audit_regressions_under_libc_so() {
    let output = common::run_fixture("security", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "security ok\n");
}
