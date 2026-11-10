mod common;

#[test]
fn process_primitives_under_libc_so() {
    let output = common::run_fixture("process", &[]);

    assert!(
        output.status.success(),
        "process_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "process ok\n");
}
