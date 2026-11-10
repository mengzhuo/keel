// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::process::Command;

#[test]
fn static_hello_links_against_libc_a() {
    let hello_bin = common::compile_static("hello.c", "hello_static");

    let file_out = Command::new("file")
        .arg(&hello_bin)
        .output()
        .expect("file command failed");
    let file_info = String::from_utf8_lossy(&file_out.stdout);
    assert!(
        file_info.contains("statically linked"),
        "binary is not statically linked: {}",
        file_info
    );

    let output = Command::new(&hello_bin)
        .output()
        .expect("failed to run hello_static");
    assert!(
        output.status.success(),
        "hello_static exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");

    // The standalone loader maps static ELF itself.  It must translate
    // AT_PHDR, map BSS, and clear the loader's thread pointer before the
    // target's startup runs.
    let output = Command::new(env!("CARGO_BIN_EXE_loader"))
        .arg(&hello_bin)
        .output()
        .expect("failed to run loader");
    assert!(
        output.status.success(),
        "loader hello_static exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}
