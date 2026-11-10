mod common;

#[test]
fn iconv_functions_under_libc_so() {
    let output = common::run_fixture("iconv", &[]);

    assert!(
        output.status.success(),
        "iconv_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "iconv ok\n");
}
