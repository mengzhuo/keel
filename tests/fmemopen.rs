// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn fmemopen_zero_capacity_under_libc_so() {
    let output = common::run_fixture("fmemopen_zero", &[]);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "fmemopen zero ok\n"
    );
}
