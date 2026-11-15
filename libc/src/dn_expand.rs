// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// musl src/network/dn_expand.c
#[no_mangle]
pub unsafe extern "C" fn dn_expand(
    base: *const u8,
    end: *const u8,
    src: *const u8,
    dest: *mut c_char,
    space: c_int,
) -> c_int {
    // `space` includes the terminating NUL.  musl rejects non-positive
    // sizes and caps the output at 254 bytes; negative values must not be
    // cast to usize.
    if space <= 0 {
        return -1;
    }
    let blen = match (end as usize).checked_sub(base as usize) {
        Some(n) => n,
        None => return -1,
    };
    let msg = match InBuf::new(base, blen) {
        Some(msg) => msg,
        None => return -1,
    };
    // `src == end` has no byte to parse; `src > end` is invalid too.
    let src_off = match (src as usize).checked_sub(base as usize) {
        Some(off) if off < blen => off,
        _ => return -1,
    };
    let cap = if space > 254 { 254 } else { space as usize };
    let mut out = match OutBuf::new(dest as *mut u8, cap) {
        Some(out) => out,
        None => return -1,
    };
    let max_steps = blen / 2 + 1;
    match dns_expand_name_at(&msg, src_off, &mut out, max_steps, usize::MAX) {
        Some(consumed) if consumed <= c_int::MAX as usize => consumed as c_int,
        _ => -1,
    }
}
