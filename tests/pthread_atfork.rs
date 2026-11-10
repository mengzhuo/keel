mod common;

#[test]
fn pthread_atfork_handlers_fire_in_order() {
    let output = common::run_fixture("pthread_atfork", &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "pthread_atfork_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        stdout,
        stderr
    );
    assert_eq!(
        stdout, "CBA\nABC\nCBA\nABC\n",
        "unexpected output: {}",
        stdout
    );
}
