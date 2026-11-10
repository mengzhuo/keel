mod common;

#[test]
fn ldso_runs_real_printf_binary() {
    let output = common::run_named("hello", "hello", &[]);

    assert!(
        output.status.success(),
        "hello exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}
