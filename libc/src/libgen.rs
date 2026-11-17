// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// libgen: basename / dirname
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dirname(s: *mut c_char) -> *mut c_char {
    if s.is_null() || *s == 0 {
        return b".\0".as_ptr() as *mut c_char;
    }
    let mut i = strlen(s as *const c_char);
    i = i.wrapping_sub(1);
    // skip trailing slashes
    while i > 0 && *s.add(i) == b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 && *s.add(0) == b'/' as c_char {
        return b"/\0".as_ptr() as *mut c_char;
    }
    // skip non-slashes
    while i > 0 && *s.add(i) != b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 {
        if *s.add(0) == b'/' as c_char {
            return b"/\0".as_ptr() as *mut c_char;
        }
        return b".\0".as_ptr() as *mut c_char;
    }
    // skip slashes
    while i > 0 && *s.add(i - 1) == b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 {
        return b"/\0".as_ptr() as *mut c_char;
    }
    *s.add(i) = 0;
    s
}

#[no_mangle]
pub unsafe extern "C" fn basename(s: *mut c_char) -> *mut c_char {
    if s.is_null() || *s == 0 {
        return b".\0".as_ptr() as *mut c_char;
    }
    let mut i = strlen(s as *const c_char);
    i = i.wrapping_sub(1);
    // strip trailing slashes
    while i > 0 && *s.add(i) == b'/' as c_char {
        *s.add(i) = 0;
        i = i.wrapping_sub(1);
    }
    // find last slash
    while i > 0 && *s.add(i - 1) != b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    s.add(i)
}

