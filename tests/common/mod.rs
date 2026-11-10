// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared fixture build/run helpers for the integration tests.
//
// Every C fixture is compiled as a PIE against the in-tree crabc libc and
// loader, then run with target/debug as LD_LIBRARY_PATH.  Tests only provide
// the fixture name and their own extra compiler flags or assertions.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn debug_dir() -> PathBuf {
    manifest_dir().join("target/debug")
}

pub fn fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures")
}

pub fn include_dir() -> PathBuf {
    manifest_dir().join("include")
}

pub fn require_runtime() {
    let ldso = debug_dir().join("libldso.so");
    let libc = debug_dir().join("libc.so");
    assert!(ldso.exists(), "libldso.so not found at {}", ldso.display());
    assert!(libc.exists(), "libc.so not found at {}", libc.display());
}

fn musl_gcc() -> Command {
    let mut command = Command::new("musl-gcc");
    command
        .arg("-fPIE")
        .arg("-pie")
        .arg("-I")
        .arg(include_dir())
        .arg("-Wl,--dynamic-linker")
        .arg(debug_dir().join("libldso.so"))
        .arg("-L")
        .arg(debug_dir())
        .arg("-Wl,--allow-shlib-undefined");
    command
}

pub fn compile_file(src: &Path, bin: &Path, extra: &[&str]) -> PathBuf {
    require_runtime();
    let mut command = musl_gcc();
    command.args(extra).arg(src).arg("-lc").arg("-o").arg(bin);
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to run musl-gcc for {}: {error}", src.display()));
    assert!(
        status.success(),
        "musl-gcc compilation failed for {}",
        src.display()
    );
    bin.to_path_buf()
}

pub fn compile_fixture(name: &str, extra: &[&str]) -> PathBuf {
    let src = fixtures_dir().join(format!("{name}_test.c"));
    let bin = fixtures_dir().join(format!("{name}_test"));
    compile_file(&src, &bin, extra)
}

/// Build a static fixture against libc.a using the host musl CRT objects.
/// The result runs both directly and through the standalone `loader`.
pub fn compile_static(src_name: &str, bin_name: &str) -> PathBuf {
    require_runtime();
    let src = fixtures_dir().join(src_name);
    let bin = fixtures_dir().join(bin_name);
    let libc_a = debug_dir().join("libc.a");
    assert!(libc_a.exists(), "libc.a not found at {}", libc_a.display());

    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "riscv64") {
        "riscv64"
    } else {
        "x86_64"
    };
    let musl_lib = format!("/usr/lib/{arch}-linux-musl");
    let status = Command::new("musl-gcc")
        .args([
            "-static",
            "-nostdlib",
            "-fno-stack-protector",
            &format!("{musl_lib}/crt1.o"),
            &format!("{musl_lib}/crti.o"),
            src.to_str().unwrap(),
            libc_a.to_str().unwrap(),
            &format!("{musl_lib}/crtn.o"),
            "-o",
            bin.to_str().unwrap(),
        ])
        .status()
        .unwrap_or_else(|error| panic!("failed to run musl-gcc for {}: {error}", src.display()));
    assert!(status.success(), "static link failed for {}", src.display());
    bin
}

pub fn run_binary(bin: &Path) -> Output {
    Command::new(bin)
        .env("LD_LIBRARY_PATH", debug_dir())
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", bin.display()))
}

pub fn run_fixture(name: &str, extra: &[&str]) -> Output {
    let bin = compile_fixture(name, extra);
    let output = run_binary(&bin);
    assert!(
        output.status.success(),
        "{name}_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

pub fn run_named(src_name: &str, bin_name: &str, extra: &[&str]) -> Output {
    run_named_with_env(src_name, bin_name, extra, &[])
}

pub fn run_named_with_env(
    src_name: &str,
    bin_name: &str,
    extra: &[&str],
    envs: &[(&str, &str)],
) -> Output {
    let src = fixtures_dir().join(format!("{src_name}.c"));
    let bin = fixtures_dir().join(bin_name);
    compile_file(&src, &bin, extra);
    let mut command = Command::new(&bin);
    command.env("LD_LIBRARY_PATH", debug_dir());
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", bin.display()));
    assert!(
        output.status.success(),
        "{src_name} exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
