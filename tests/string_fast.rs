mod common;

#[test]
fn string_fast_paths_under_libc_so() {
    let output = common::run_fixture("string_fast", &["-O2"]);

    assert!(
        output.status.success(),
        "string_fast_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "string fast ok\n");
}
