// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

mod common;

#[test]
fn group_members_are_not_truncated() {
    let output = common::run_fixture("group_many", &[]);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "members=100 last=user099\n"
    );
}
