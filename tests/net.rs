mod common;

#[test]
fn net_functions_under_libc_so() {
    let output = common::run_fixture("net", &[]);

    assert!(
        output.status.success(),
        "net_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "net ok\n");
}
