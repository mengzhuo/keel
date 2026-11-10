// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn new_functions_under_libc_so() {
    let output = common::run_fixture("new_functions", &["-D_GNU_SOURCE"]);

    assert!(
        output.status.success(),
        "new_functions_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "new_functions ok\n"
    );
}
