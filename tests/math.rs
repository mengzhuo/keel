// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn math_functions_under_libc_so() {
    let mut extra = vec!["-fno-builtin", "-frounding-math"];
    if cfg!(target_arch = "x86_64") {
        extra.push("-mlong-double-64");
    }
    let output = common::run_fixture("math", &extra);

    assert!(
        output.status.success(),
        "math_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "math ok\n");
}
