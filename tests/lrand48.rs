// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn lrand48_musl_vectors_under_libldso() {
    let output = common::run_fixture("lrand48", &["-D_GNU_SOURCE"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "lrand48_test exited with {:?}, stderr: {}",
        output.status.code(),
        stderr
    );
    assert_eq!(stdout.trim(), "lrand48 ok", "expected lrand48 ok in: {}", stdout);
}
