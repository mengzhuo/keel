// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

use std::process::Command;

#[test]
fn dlopen_musl_semantics() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest_dir.join("tests/fixtures");
    let include = manifest_dir.join("include");
    let debug = manifest_dir.join("target/debug");
    let ldso = debug.join("libldso.so");
    let libc = debug.join("libc.so");
    assert!(ldso.exists(), "libldso.so not found");
    assert!(libc.exists(), "libc.so not found");

    let dep_src = fixtures.join("dlopen_dep.c");
    let dep = fixtures.join("libdlopen_dep.so");
    let a_src = fixtures.join("dlopen_a.c");
    let a = fixtures.join("libdlopen_a.so");
    let b_src = fixtures.join("dlopen_b.c");
    let b = fixtures.join("libdlopen_b.so");
    let ghost = fixtures.join("libdlopen_ghost.so");
    let missing = fixtures.join("libdlopen_missing.so");
    let main_src = fixtures.join("dlopen_semantics_test.c");
    let main = fixtures.join("dlopen_semantics_test");

    let status = Command::new("musl-gcc")
        .args(["-shared", "-fPIC", "-Wl,-soname,libdlopen_dep.so"])
        .arg(&dep_src)
        .arg("-o")
        .arg(&dep)
        .status()
        .expect("failed to run musl-gcc for libdlopen_dep.so");
    assert!(status.success(), "building libdlopen_dep.so failed");

    let status = Command::new("musl-gcc")
        .args([
            "-shared",
            "-fPIC",
            "-Wl,-soname,libdlopen_a.so",
            "-Wl,-rpath,$ORIGIN",
        ])
        .arg(&a_src)
        .args(["-L"])
        .arg(&fixtures)
        .args(["-Wl,--no-as-needed", "-ldlopen_dep"])
        .args(["-L"])
        .arg(&debug)
        .args(["-lc", "-Wl,--allow-shlib-undefined"])
        .arg("-o")
        .arg(&a)
        .status()
        .expect("failed to run musl-gcc for libdlopen_a.so");
    assert!(status.success(), "building libdlopen_a.so failed");

    let status = Command::new("musl-gcc")
        .args(["-shared", "-nostdlib", "-fPIC", "-Wl,-soname,libdlopen_b.so"])
        .arg(&b_src)
        .arg("-o")
        .arg(&b)
        .status()
        .expect("failed to run musl-gcc for libdlopen_b.so");
    assert!(status.success(), "building libdlopen_b.so failed");

    // Build a DSO with a DT_NEEDED entry, then remove the dependency so
    // dlopen() has to fail gracefully and roll back.
    let status = Command::new("musl-gcc")
        .args(["-shared", "-nostdlib", "-fPIC", "-Wl,-soname,libdlopen_ghost.so"])
        .arg(&dep_src)
        .arg("-o")
        .arg(&ghost)
        .status()
        .expect("failed to run musl-gcc for libdlopen_ghost.so");
    assert!(status.success(), "building libdlopen_ghost.so failed");

    let status = Command::new("musl-gcc")
        .args([
            "-shared",
            "-fPIC",
            "-Wl,-soname,libdlopen_missing.so",
            "-Wl,-rpath,$ORIGIN",
            "-Wl,--no-as-needed",
        ])
        .arg(&b_src)
        .args(["-L"])
        .arg(&fixtures)
        .arg("-ldlopen_ghost")
        .args(["-L"])
        .arg(&debug)
        .args(["-lc", "-Wl,--allow-shlib-undefined"])
        .arg("-o")
        .arg(&missing)
        .status()
        .expect("failed to run musl-gcc for libdlopen_missing.so");
    assert!(status.success(), "building libdlopen_missing.so failed");
    std::fs::remove_file(&ghost).expect("failed to remove libdlopen_ghost.so");

    let status = Command::new("musl-gcc")
        .args(["-O2", "-fPIE", "-pie", "-I"])
        .arg(&include)
        .args(["-Wl,--dynamic-linker"])
        .arg(&ldso)
        .args(["-Wl,--export-dynamic", "-L"])
        .arg(&debug)
        .arg(&main_src)
        .args(["-lc", "-Wl,--allow-shlib-undefined", "-o"])
        .arg(&main)
        .status()
        .expect("failed to run musl-gcc for dlopen_semantics_test");
    assert!(status.success(), "building dlopen_semantics_test failed");

    // Deliberately do not put the fixtures directory on LD_LIBRARY_PATH:
    // libdlopen_a.so must find libdlopen_dep.so through its $ORIGIN RUNPATH.
    let output = Command::new(&main)
        .env("LD_LIBRARY_PATH", debug.to_str().unwrap())
        .arg(&missing)
        .arg(&a)
        .arg(&b)
        .output()
        .expect("failed to run dlopen_semantics_test");

    assert!(
        output.status.success(),
        "dlopen_semantics_test exited with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "dlopen semantics ok\n");
}
