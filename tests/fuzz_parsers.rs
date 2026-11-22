// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn parser_fuzz_smoke_under_libc_so() {
    let output = common::run_fixture("fuzz_parsers", &[]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "fuzz ok\n");
}
