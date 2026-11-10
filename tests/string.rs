mod common;

#[test]
fn string_functions_under_libc_so() {
    let output = common::run_fixture("string", &[]);

    assert!(
        output.status.success(),
        "string_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "string ok\n");
}
