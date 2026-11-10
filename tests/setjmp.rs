mod common;

#[test]
fn setjmp_longjmp_under_libc_so() {
    let output = common::run_fixture("setjmp", &[]);

    assert!(
        output.status.success(),
        "setjmp_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "setjmp ok\n");
}
