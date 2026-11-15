// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

use std::process::Command;

#[test]
fn loader_runs_tiny_pie() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest_dir.join("tests/fixtures");
    let out = manifest_dir.join("target/fixtures");
    std::fs::create_dir_all(&out).expect("create target/fixtures");
    let tiny_src = fixtures.join("tiny.c");
    let tiny_bin = out.join("tiny");

    let status = Command::new("musl-gcc")
        .args([
            "-fPIE",
            "-pie",
            "-nostdlib",
            "-nostartfiles",
            tiny_src.to_str().unwrap(),
            "-Wl,--allow-shlib-undefined",
            "-o",
            tiny_bin.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run musl-gcc");
    assert!(status.success(), "musl-gcc compilation failed");

    let loader_bin = env!("CARGO_BIN_EXE_keel");
    let output = Command::new(loader_bin)
        .arg(&tiny_bin)
        .output()
        .expect("failed to run loader");

    assert!(
        output.status.success(),
        "loader exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello\n");
}
