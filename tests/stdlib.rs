mod common;

#[test]
fn stdlib_functions_under_libc_so() {
    let output = common::run_fixture("stdlib", &[]);

    assert!(
        output.status.success(),
        "stdlib_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "stdlib ok\n");
}
