// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::path::PathBuf;
use std::process::Command;

fn build_tls_dso() -> PathBuf {
    let src = common::fixtures_dir().join("tlsdesc_dso.c");
    let bin = common::build_fixtures_dir().join("libtlsdesc.so");
    let mut args = vec![
        "-shared".to_string(),
        "-fPIC".to_string(),
        "-O2".to_string(),
        "-I".to_string(),
        common::include_dir().to_str().unwrap().to_string(),
        src.to_str().unwrap().to_string(),
        "-o".to_string(),
        bin.to_str().unwrap().to_string(),
    ];
    // x86_64 defaults to the fs-relative dialect, so request TLSDESC
    // explicitly; the other architectures emit it by default.
    #[cfg(target_arch = "x86_64")]
    args.push("-mtls-dialect=gnu2".to_string());
    let status = Command::new("musl-gcc")
        .args(&args)
        .status()
        .expect("failed to run musl-gcc for libtlsdesc.so");
    assert!(status.success(), "building libtlsdesc.so failed");
    bin
}

#[test]
fn tlsdesc_reaches_early_and_late_threads() {
    // A thread created before dlopen must still see the module's initial
    // .tdata and keep its own copy of the variable.  aarch64 and loongarch64
    // use TLSDESC by default; x86_64 is built with -mtls-dialect=gnu2 above
    // so this also covers R_X86_64_TLSDESC.
    let dso = build_tls_dso();
    let output = common::run_named_with_env(
        "tlsdesc_test",
        "tlsdesc_test",
        &["-pthread"],
        &[("TLSDESC_DSO", dso.to_str().unwrap())],
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tlsdesc ok\n");
}
