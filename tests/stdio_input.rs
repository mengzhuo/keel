use std::fs::File;
use std::io::Write;
mod common;

use std::process::Command;

#[test]
fn stdio_input_functions_under_libc_so() {
    let bin = common::compile_fixture("stdio_input", &[]);

    let input_path = common::fixtures_dir().join("stdio_input_test.txt");
    {
        let mut f = File::create(&input_path).expect("create input file");
        f.write_all(b"hi\nxy\nz").expect("write input");
    }

    let output = Command::new(&bin)
        .env("LD_LIBRARY_PATH", common::debug_dir().to_str().unwrap())
        .stdin(File::open(&input_path).expect("open input"))
        .output()
        .expect("failed to run stdio_input_test");

    assert!(
        output.status.success(),
        "stdio_input_test exited with {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "stdio input ok\n");
}
