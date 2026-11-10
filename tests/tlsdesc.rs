// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::path::PathBuf;
use std::process::Command;

fn build_tls_dso() -> PathBuf {
    let src = common::fixtures_dir().join("tlsdesc_dso.c");
    let bin = common::fixtures_dir().join("libtlsdesc.so");
    let status = Command::new("musl-gcc")
        .args([
            "-shared",
            "-fPIC",
            "-O2",
            "-I",
            common::include_dir().to_str().unwrap(),
            src.to_str().unwrap(),
            "-o",
            bin.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run musl-gcc for libtlsdesc.so");
    assert!(status.success(), "building libtlsdesc.so failed");
    bin
}

#[test]
fn tlsdesc_reaches_early_and_late_threads() {
    // A thread created before dlopen must still see the module's initial
    // .tdata and keep its own copy of the variable.  On aarch64 this goes
    // through the dynamic TLSDESC resolver.
    let dso = build_tls_dso();
    let output = common::run_named_with_env(
        "tlsdesc_test",
        "tlsdesc_test",
        &["-pthread"],
        &[("TLSDESC_DSO", dso.to_str().unwrap())],
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tlsdesc ok\n");
}
