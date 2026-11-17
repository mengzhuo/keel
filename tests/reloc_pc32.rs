// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

#![cfg(target_arch = "x86_64")]

mod common;

use std::path::PathBuf;
use std::process::Command;

fn build_pc32_dso() -> PathBuf {
    let src = common::fixtures_dir().join("pc32_dso.c");
    let bin = common::build_fixtures_dir().join("libpc32.so");
    let status = Command::new("musl-gcc")
        .args([
            "-shared",
            "-fPIC",
            "-nostdlib",
            src.to_str().unwrap(),
            "-o",
            bin.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run musl-gcc for libpc32.so");
    assert!(status.success(), "building libpc32.so failed");
    bin
}

#[test]
fn pc32_dynamic_relocation_is_applied() {
    // R_X86_64_PC32 in .rela.dyn (musl's REL_OFFSET32).
    let dso = build_pc32_dso();
    let output = common::run_named_with_env(
        "pc32_test",
        "pc32_test",
        &["-Wl,--export-dynamic"],
        &[("PC32_DSO", dso.to_str().unwrap())],
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "pc32 ok\n");
}
