// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::process::Command;

#[test]
fn args_and_env_are_not_truncated() {
    let bin = common::compile_fixture("argenv", &[]);
    let mut command = Command::new(&bin);
    command.env_clear();
    for i in 1..=300 {
        command.arg(format!("arg{i}"));
    }
    for i in 0..600 {
        command.env(format!("VAR{i}"), "x");
    }
    command.env("LD_LIBRARY_PATH", common::debug_dir());
    let output = command.output().expect("failed to run argenv_test");
    assert!(
        output.status.success(),
        "argenv_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "301 601 arg1 arg300\n"
    );
}
