// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn time_regressions_under_libc_so() {
    let output = common::run_fixture("time_regressions", &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "time_regressions_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert_eq!(stdout, "time regressions ok\n");
}
