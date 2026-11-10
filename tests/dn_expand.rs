// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn dn_expand_under_libc_so() {
    let output = common::run_fixture("dn_expand", &["-D_GNU_SOURCE"]);

    assert!(
        output.status.success(),
        "dn_expand_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
