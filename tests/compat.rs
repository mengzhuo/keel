mod common;

#[test]
fn musl_compat_symbols_work() {
    let output = common::run_fixture("compat", &["-D_GNU_SOURCE", "-lpthread", "-lm"]);

    assert!(
        output.status.success(),
        "compat_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "compat ok\n"
    );
}
