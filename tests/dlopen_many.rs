// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::fs;
use std::process::Command;

#[test]
fn more_than_256_shared_objects_can_be_loaded() {
    let manifest = common::manifest_dir();
    let dir = manifest.join("target/dlopen-many");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create dlopen-many dir");

    let base = dir.join("libbase.so");
    let status = Command::new("musl-gcc")
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            "-I",
            common::include_dir().to_str().unwrap(),
            manifest.join("tests/fixtures/libfoo.c").to_str().unwrap(),
            "-o",
            base.to_str().unwrap(),
        ])
        .status()
        .expect("failed to build libbase.so");
    assert!(status.success(), "building libbase.so failed");

    for i in 1..=300 {
        fs::copy(&base, dir.join(format!("libmany{i}.so"))).expect("copy DSO");
    }

    let bin = common::compile_fixture("dlopen_many", &[]);
    let output = Command::new(&bin)
        .arg(&dir)
        .arg("300")
        .env("LD_LIBRARY_PATH", common::debug_dir())
        .output()
        .expect("failed to run dlopen_many_test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "dlopen_many_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert_eq!(stdout, "loaded=300\n");
    let _ = fs::remove_dir_all(&dir);
}
