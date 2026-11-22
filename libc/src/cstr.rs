// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Internal helpers for NUL-terminated C strings.  They are not part of the
// ABI; they keep pointer arithmetic and c_char signedness out of call sites.

/// Compare a NUL-terminated string with a known byte sequence (typically a
/// Rust literal).  Unlike `strcmp`, the expected side does not need a
/// trailing NUL and the comparison stops after `want.len()` bytes.
#[inline]
pub unsafe fn cstr_eq(s: *const u8, want: &[u8]) -> bool {
    if s.is_null() {
        return false;
    }
    let mut i = 0usize;
    while i < want.len() {
        if *s.add(i) != want[i] {
            return false;
        }
        i += 1;
    }
    *s.add(want.len()) == 0
}

/// Borrow a NUL-terminated string as bytes, excluding the terminator.  The
/// caller must keep the string alive for the lifetime of the slice.
#[inline]
pub unsafe fn cstr_bytes<'a>(s: *const c_char) -> &'a [u8] {
    if s.is_null() {
        return &[];
    }
    let len = strlen(s);
    core::slice::from_raw_parts(s as *const u8, len)
}

/// True when `s` contains `needle` under ASCII case folding.
#[inline]
pub unsafe fn cstr_contains_ci(s: *const u8, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    cstr_bytes(s as *const c_char)
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}
