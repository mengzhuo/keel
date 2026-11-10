mod common;
use std::sync::Mutex;

static PTHREAD_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn pthread_functions_under_libc_so() {
    let _guard = PTHREAD_TEST_LOCK.lock();
    let output = common::run_fixture("pthread", &[]);

    assert!(
        output.status.success(),
        "pthread_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "pthread ok\n");
}

#[test]
fn pthread_full_test() {
    let _guard = PTHREAD_TEST_LOCK.lock();
    let output = common::run_fixture("pthread_full", &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "pthread_full_test exited with {:?}, stdout: {}, stderr: {}",
        output.status.code(), stdout, stderr
    );
    assert!(stdout.contains("pthread_full ok"), "unexpected output: {}", stdout);
}
