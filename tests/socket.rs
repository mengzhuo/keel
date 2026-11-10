mod common;

#[test]
fn socket_functions_under_libc_so() {
    let output = common::run_fixture("socket", &[]);

    assert!(
        output.status.success(),
        "socket_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "socket ok\n");
}
