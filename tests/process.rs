// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn process_primitives_under_libc_so() {
    let output = common::run_fixture("process", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "process ok\n");
}
