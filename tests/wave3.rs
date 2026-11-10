// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn wave3_regressions_under_libc_so() {
    let output = common::run_fixture("wave3", &[]);

    assert!(
        output.status.success(),
        "wave3_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "wave3 ok\n");
}
