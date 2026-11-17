// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn syscall_sign_extend_under_libc() {
    let output = common::run_fixture("syscall", &["-D_GNU_SOURCE"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("OK"), "expected OK in stdout: {}", stdout);
}
