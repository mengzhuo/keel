mod common;

#[test]
fn stdio_full_functions_under_libc_so() {
    let output = common::run_fixture("stdio_full", &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "stdio_full_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert!(
        stdout.contains("stdio full ok"),
        "expected 'stdio full ok' in stdout, got: {}",
        stdout
    );
}
