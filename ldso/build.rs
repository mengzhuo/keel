// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

fn main() {
    println!("cargo:rustc-cdylib-link-arg=-nostartfiles");
    println!("cargo:rustc-cdylib-link-arg=-nostdlib");
    println!("cargo:rustc-cdylib-link-arg=-e");
    println!("cargo:rustc-cdylib-link-arg=_start");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-Bsymbolic");
}
