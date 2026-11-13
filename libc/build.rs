// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

fn main() {
    // Shrink .rela.dyn (about 1,070 relative relocations) to a DT_RELR
    // bitmap; the dynamic linker applies both forms.
    println!("cargo:rustc-cdylib-link-arg=-Wl,-z,pack-relative-relocs");
    // Bind references to definitions inside libc.so itself.  Without this,
    // hundreds of GOT entries stay as R_*_GLOB_DAT and require symbol lookup
    // on every process startup.  The weak loader hooks must remain
    // preemptible so libldso.so can provide the real implementations.
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let list = format!("{out}/keel-dynamic-list");
    std::fs::write(
        &list,
        "{\n\
         __keel_create_thread_tls;\n\
         __keel_init_thread_tls_at;\n\
         __keel_tls_block_size;\n\
         __keel_tls_base_offset;\n\
         };\n",
    )
    .expect("write dynamic list");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-Bsymbolic");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--dynamic-list={list}");
}
