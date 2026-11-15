// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn ldso_runs_real_printf_binary() {
    let output = common::run_named("hello", "hello", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}
