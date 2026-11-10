mod common;

#[test]
fn locale_functions_under_libc_so() {
    let output = common::run_fixture("locale", &[]);

    assert!(
        output.status.success(),
        "locale_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "locale ok\n");
}
