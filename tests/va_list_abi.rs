mod common;

#[test]
fn va_list_abi_under_libldso() {
    let output = common::run_fixture("va_list_abi", &["-D_GNU_SOURCE"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "va_list_abi_test exited with {:?}, stderr: {}",
        output.status.code(),
        stderr
    );
    assert!(stdout.contains("OK"), "expected OK in: {}", stdout);
}
