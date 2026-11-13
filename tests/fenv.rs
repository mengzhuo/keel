// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn fenv_round_and_except_under_libc() {
    let output = common::run_fixture("fenv", &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "fenv_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert_eq!(stdout, "OK\n", "unexpected output: {}", stdout);
}
