// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn printf_bounds_under_libc_so() {
    let output = common::run_fixture("printf_bounds", &[]);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "printf bounds ok\n"
    );
}
