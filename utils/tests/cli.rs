// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_keel-utils"))
        .args(args)
        .output()
        .expect("failed to run keel-utils")
}

#[test]
fn getconf_reports_page_size_and_path() {
    let out = run(&["getconf", "PAGESIZE"]);
    assert!(out.status.success());
    let value: usize = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap();
    assert!(value >= 4096);

    let out = run(&["getconf", "PATH"]);
    assert!(out.status.success());
    let path = String::from_utf8_lossy(&out.stdout);
    assert!(path.contains("/bin"));
}

#[test]
fn getent_reads_passwd() {
    let out = run(&["getent", "passwd", "root"]);
    assert!(out.status.success());
    let line = String::from_utf8_lossy(&out.stdout);
    assert!(line.starts_with("root:"), "unexpected output: {line}");
}

#[test]
fn iconv_converts_utf8_to_utf16le() {
    let out = Command::new(env!("CARGO_BIN_EXE_keel-utils"))
        .args(["iconv", "-f", "UTF-8", "-t", "UTF-16LE"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(b"abc")?;
            child.wait_with_output()
        })
        .expect("failed to run iconv");
    assert!(out.status.success());
    assert_eq!(out.stdout, [0x61, 0x00, 0x62, 0x00, 0x63, 0x00]);
}
