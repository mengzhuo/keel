mod common;

#[test]
fn statvfs_under_libc_so() {
    let output = common::run_fixture("statvfs", &[]);

    assert!(
        output.status.success(),
        "statvfs_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "statvfs ok\n"
    );
}
