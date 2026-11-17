// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Wide char functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn wcscasecmp(s1: *const u32, s2: *const u32) -> i32 {
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        let l1 = if c1 >= 'A' as u32 && c1 <= 'Z' as u32 { c1 + 32 } else { c1 };
        let l2 = if c2 >= 'A' as u32 && c2 <= 'Z' as u32 { c2 + 32 } else { c2 };
        if l1 != l2 { return (l1 as i32) - (l2 as i32); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscasecmp_l(s1: *const u32, s2: *const u32, _loc: *mut core::ffi::c_void) -> i32 {
    wcscasecmp(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn wcsncasecmp(s1: *const u32, s2: *const u32, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        let l1 = if c1 >= 'A' as u32 && c1 <= 'Z' as u32 { c1 + 32 } else { c1 };
        let l2 = if c2 >= 'A' as u32 && c2 <= 'Z' as u32 { c2 + 32 } else { c2 };
        if l1 != l2 { return (l1 as i32) - (l2 as i32); }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn wcsncasecmp_l(s1: *const u32, s2: *const u32, n: usize, _loc: *mut core::ffi::c_void) -> i32 {
    wcsncasecmp(s1, s2, n)
}

#[no_mangle]
pub unsafe extern "C" fn wcscoll(s1: *const u32, s2: *const u32) -> i32 {
    // musl's C locale compares wide characters by codepoint.
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        if c1 != c2 { return (c1 as i32) - (c2 as i32); }
        i += 1;
    }
}

unsafe fn wc_in_set(c: u32, set: *const u32) -> bool {
    let mut i = 0usize;
    while *set.add(i) != 0 {
        if *set.add(i) == c { return true; }
        i += 1;
    }
    false
}

#[no_mangle]
pub unsafe extern "C" fn wcstok(s: *mut u32, sep: *const u32, p: *mut *mut u32) -> *mut u32 {
    if sep.is_null() || p.is_null() { return core::ptr::null_mut(); }
    let mut src = if s.is_null() {
        if (*p).is_null() { return core::ptr::null_mut(); }
        *p
    } else { s };
    while *src != 0 && wc_in_set(*src, sep) { src = src.add(1); }
    if *src == 0 { return core::ptr::null_mut(); }
    let start = src;
    while *src != 0 && !wc_in_set(*src, sep) { src = src.add(1); }
    if *src != 0 {
        *src = 0;
        *p = src.add(1);
    } else {
        *p = src;
    }
    start
}

#[no_mangle]
pub unsafe extern "C" fn wcswcs(haystack: *const u32, needle: *const u32) -> *mut u32 {
    if *needle == 0 { return haystack as *mut u32; }
    let mut i = 0;
    while *haystack.add(i) != 0 {
        let mut j = 0;
        while *needle.add(j) != 0 && *haystack.add(i + j) == *needle.add(j) {
            j += 1;
        }
        if *needle.add(j) == 0 { return haystack.add(i) as *mut u32; }
        i += 1;
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wcswidth(s: *const u32, n: usize) -> i32 {
    let mut width = 0i32;
    let mut i = 0usize;
    while i < n {
        let c = *s.add(i);
        if c == 0 { break; }
        let w = wcwidth(c);
        if w < 0 { return w; }
        width += w;
        i += 1;
    }
    width
}

#[no_mangle]
pub unsafe extern "C" fn wcsftime(s: *mut u32, maxsize: usize, fmt: *const u32, tm: *const c_void) -> usize {
    if s.is_null() || fmt.is_null() || maxsize == 0 { return 0; }
    // Convert the wide format string to multibyte, let strftime do the work,
    // then convert the result back. Both buffers scale with the inputs.
    let wlen = wcslen(fmt as *const wchar_t);
    let cfmt_cap = match wlen.checked_mul(4).and_then(|v| v.checked_add(1)) {
        Some(v) => v,
        None => return 0,
    };
    let cfmt = crate::malloc(cfmt_cap) as *mut u8;
    if cfmt.is_null() { ERRNO!() = ENOMEM; return 0; }
    let mut o = 0usize;
    let mut i = 0usize;
    let mut st: c_uint = 0;
    while *fmt.add(i) != 0 {
        let n = wcrtomb(
            cfmt.add(o) as *mut c_char,
            *fmt.add(i) as c_int,
            &mut st as *mut c_uint,
        );
        if n == !0usize || o + n >= cfmt_cap {
            crate::free(cfmt as *mut c_void);
            return 0;
        }
        o += n;
        i += 1;
    }
    *cfmt.add(o) = 0;
    let out_cap = match maxsize.checked_mul(4).and_then(|v| v.checked_add(1)) {
        Some(v) => v,
        None => { crate::free(cfmt as *mut c_void); return 0; }
    };
    let out = crate::malloc(out_cap) as *mut u8;
    if out.is_null() {
        crate::free(cfmt as *mut c_void);
        ERRNO!() = ENOMEM;
        return 0;
    }
    let n = strftime(out as *mut c_char, out_cap, cfmt as *const c_char, tm as *const tm);
    crate::free(cfmt as *mut c_void);
    if n == 0 {
        crate::free(out as *mut c_void);
        return 0;
    }
    let mut ws = 0usize;
    let mut p = 0usize;
    while p < n && ws + 1 < maxsize {
        let mut wc: c_int = 0;
        let mut st2: c_uint = 0;
        let r = mbrtowc(&mut wc, out.add(p) as *const c_char, n - p, &mut st2);
        if r == !0usize {
            crate::free(out as *mut c_void);
            return 0;
        }
        if r == 0 || r == !1usize { break; }
        *s.add(ws) = wc as u32;
        ws += 1;
        p += r;
    }
    *s.add(ws) = 0;
    crate::free(out as *mut c_void);
    ws
}

#[no_mangle]
pub unsafe extern "C" fn mbsnrtowcs(
    dst: *mut u32,
    src: *mut *const u8,
    nms: usize,
    len: usize,
    st: *mut core::ffi::c_void,
) -> usize {
    if src.is_null() { return 0; }
    let mut s = *src;
    if s.is_null() { return 0; }
    let state = st as *mut c_uint;
    let limit = if dst.is_null() { usize::MAX } else { len };
    let mut count = 0usize;
    let mut n = nms;
    while count < limit && n > 0 {
        let c = *s;
        if c == 0 {
            if !dst.is_null() {
                if count < len { *dst.add(count) = 0; }
                *src = core::ptr::null();
            }
            return count;
        }
        let mut wc: c_int = 0;
        let r = mbrtowc(&mut wc, s as *const c_char, n, state);
        if r == !0usize {
            *src = s;
            return !0usize;
        }
        if r == !1usize { break; }
        if !dst.is_null() { *dst.add(count) = wc as u32; }
        count += 1;
        s = s.add(r);
        n -= r;
    }
    *src = s;
    count
}

#[no_mangle]
pub unsafe extern "C" fn wcsnrtombs(dst: *mut u8, src: *mut *const u32, nwc: usize, len: usize, _st: *mut core::ffi::c_void) -> usize {
    // Port of musl's wcsnrtombs: nwc counts wide characters, len counts
    // output bytes, and a converted NUL terminates the source pointer.
    let mut ws = *src;
    let mut wn = nwc;
    let mut cnt = 0usize;
    let mut n = len;
    let mut d = dst;
    while !ws.is_null() && wn > 0 {
        let c = *ws;
        let mut tmp = [0u8; 8];
        let mut state: c_uint = 0;
        let l = wcrtomb(tmp.as_mut_ptr() as *mut c_char, c as c_int, &mut state);
        if l == !0usize {
            return !0usize;
        }
        if !d.is_null() {
            if l > n {
                break;
            }
            core::ptr::copy_nonoverlapping(tmp.as_ptr(), d, l);
            d = d.add(l);
            n -= l;
        }
        if c == 0 {
            ws = core::ptr::null();
            break;
        }
        ws = ws.add(1);
        wn -= 1;
        cnt += l;
    }
    if !dst.is_null() {
        *src = ws;
    }
    cnt
}

