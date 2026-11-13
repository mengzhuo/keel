// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::path::PathBuf;
use std::process::Command;

fn build_tls_dso() -> PathBuf {
    let src = common::fixtures_dir().join("vdso_tls.c");
    let bin = common::fixtures_dir().join("libvdso_tls.so");
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
        .expect("failed to run musl-gcc for libvdso_tls.so");
    assert!(status.success(), "building libvdso_tls.so failed");
    bin
}

#[test]
fn vdso_fast_paths_preserve_semantics() {
    // The fixture checks that the loader preserved AT_SYSINFO_EHDR and that
    // clock_gettime/gettimeofday/time/getcpu/getrandom still behave like the
    // syscalls, including across threads and fork.  The TLS DSO forces the
    // thread's TCB to move and may leave the thread pointer 4-byte aligned.
    let dso = build_tls_dso();
    let dso = dso.to_str().unwrap();
    let output = common::run_named_with_env(
        "vdso_test",
        "vdso_test",
        &["-pthread"],
        &[("VDSO_TLS_DSO", dso)],
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "vdso ok\n");
}

#[test]
fn vdso_fast_paths_work_in_static_binary() {
    let bin = common::compile_static("vdso_test.c", "vdso_static_test");

    let output = Command::new(&bin)
        .output()
        .expect("failed to run static vdso_test");
    assert!(
        output.status.success(),
        "static vdso_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "vdso ok\n");

    // The standalone loader must hand the static target a usable vDSO and
    // let keel build its own TCB instead of inheriting the loader's.
    let output = Command::new(env!("CARGO_BIN_EXE_keel"))
        .arg(&bin)
        .output()
        .expect("failed to run loader");
    assert!(
        output.status.success(),
        "loader vdso_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "vdso ok\n");
}
