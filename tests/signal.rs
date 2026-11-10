// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn signal_functions_under_libc_so() {
    let output = common::run_fixture("signal", &[]);

    assert!(
        output.status.success(),
        "signal_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "signal ok\n");
}
