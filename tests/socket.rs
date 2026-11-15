// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn socket_functions_under_libc_so() {
    let output = common::run_fixture("socket", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "socket ok\n");
}
