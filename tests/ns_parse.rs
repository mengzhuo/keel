// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn ns_parse_under_libc_so() {
    let output = common::run_fixture("ns_parse", &["-D_GNU_SOURCE"]);

    assert_eq!(String::from_utf8_lossy(&output.stdout), "ns_parse ok\n");
}
