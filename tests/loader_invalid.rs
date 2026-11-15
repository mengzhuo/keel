// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

use std::fs;
use std::process::Command;

fn put16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

fn put32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

fn put64(buf: &mut [u8], off: usize, v: u64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn loader_rejects_bad_phentsize() {
    let keel = common::debug_dir().join("keel");
    assert!(keel.exists(), "keel binary not found at {}", keel.display());

    let machine = if cfg!(target_arch = "x86_64") {
        62u16
    } else if cfg!(target_arch = "aarch64") {
        183u16
    } else if cfg!(target_arch = "riscv64") {
        243u16
    } else {
        return;
    };

    let mut elf = [0u8; 64];
    elf[..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2; // ELFCLASS64
    elf[5] = 1; // little-endian
    elf[6] = 1; // EV_CURRENT
    put16(&mut elf, 16, 2); // ET_EXEC
    put16(&mut elf, 18, machine);
    put32(&mut elf, 20, 1); // e_version
    put64(&mut elf, 32, 64); // e_phoff (past EOF)
    put16(&mut elf, 52, 64); // e_ehsize
    put16(&mut elf, 54, 1); // bad e_phentsize
    put16(&mut elf, 56, 1); // e_phnum

    let path = std::env::temp_dir().join(format!("keel-bad-phdr-{}", std::process::id()));
    fs::write(&path, elf).expect("write malformed ELF");
    let output = Command::new(&keel)
        .arg(&path)
        .output()
        .expect("run keel loader");
    let _ = fs::remove_file(&path);

    assert!(
        !output.status.success(),
        "keel unexpectedly accepted malformed ELF: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("e_phentsize") || stderr.contains("phdr"),
        "unexpected loader error: {stderr}"
    );
}
