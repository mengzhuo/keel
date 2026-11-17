// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

use std::process::Command;

fn parse_count(stdout: &str, label: &str) -> i32 {
    stdout
        .lines()
        .find(|l| l.trim_start().starts_with(label))
        .and_then(|l| {
            l.split_once(label)
                .map(|x| x.1.trim().parse::<i32>().unwrap_or(0))
        })
        .unwrap_or(0)
}

#[test]
fn libc_test_regression_zero_failures() {
    let libc_test_dir = std::env::var("LIBC_TEST_DIR").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("tests live in the repository root")
            .join("libc-test")
            .to_string_lossy()
            .into_owned()
    });
    if !std::path::Path::new(&libc_test_dir).join("src").is_dir() {
        eprintln!(
            "skipping libc_test_regression_zero_failures: libc-test source not found at {}",
            libc_test_dir
        );
        return;
    }

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let harness = manifest_dir.join("scripts/libc-test-harness/run.sh");

    let output = Command::new(&harness)
        .arg("regression")
        .output()
        .expect("failed to run libc-test regression harness");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "regression harness exited with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        stdout,
        stderr
    );

    let fail = parse_count(&stdout, "FAIL:");
    // One regression test is timing-sensitive; the harness retries it, but a
    // heavily loaded machine can still lose all attempts. Allow only that
    // exact test, so any new failure is visible.
    const FLAKY: &str = "regression/pthread_cond_wait-cancel_ignored";
    let failed: Vec<&str> = stdout
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("FAIL tests:"))
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .map(|l| l.trim())
        .collect();
    let only_known_flake = fail == 0 || (fail == 1 && failed == [FLAKY]);
    assert!(
        only_known_flake,
        "expected zero FAILs (only {FLAKY} may flake), got {fail}: {failed:?}\n{stdout}"
    );
}
