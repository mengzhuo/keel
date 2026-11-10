mod common;

#[test]
fn ldso_startup_argv_env() {
    let output = common::run_named_with_env(
        "startup_argv_env",
        "startup_argv_env",
        &["-D_GNU_SOURCE"],
        &[("TEST_STARTUP_VAR", "hello_world")],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "startup_argv_env exited with {:?}, stderr: {}",
        output.status.code(),
        stderr
    );
    assert!(stdout.contains("argc=1"), "expected argc=1 in: {}", stdout);
    assert!(
        !stdout.contains("argv0=") || !stdout.contains("argv0=\n"),
        "argv[0] should not be empty: {}",
        stdout
    );
    assert!(
        stdout.contains("env=hello_world"),
        "expected env=hello_world in: {}",
        stdout
    );
}
