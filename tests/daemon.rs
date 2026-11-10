mod common;

#[test]
fn daemon_double_fork_under_libldso() {
    let output = common::run_fixture("daemon", &["-D_GNU_SOURCE"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "daemon_test exited with {:?}, stderr: {}",
        output.status.code(),
        stderr
    );
    assert!(stdout.contains("OK"), "expected OK in stdout: {}", stdout);
}
