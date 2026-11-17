// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn getpw_r_under_libc_so() {
    let output = common::run_fixture("getpw_r", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "getpw_r ok\n");
}
