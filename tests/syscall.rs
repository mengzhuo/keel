mod common;

#[test]
fn syscall_sign_extend_under_libldso() {
    let output = common::run_fixture("syscall", &["-D_GNU_SOURCE"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "syscall_test exited with {:?}, stderr: {}",
        output.status.code(),
        stderr
    );
    assert!(stdout.contains("OK"), "expected OK in stdout: {}", stdout);
}
