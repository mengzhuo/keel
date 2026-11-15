// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn netdb_threads_under_libc_so() {
    let output = common::run_fixture("netdb_threads", &["-lpthread"]);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "netdb threads ok\n"
    );
}
