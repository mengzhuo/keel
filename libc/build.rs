// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

fn main() {
    // Shrink .rela.dyn (about 1,070 relative relocations) to a DT_RELR
    // bitmap; the dynamic linker applies both forms.
    println!("cargo:rustc-cdylib-link-arg=-Wl,-z,pack-relative-relocs");
}
