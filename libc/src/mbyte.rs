// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// multibyte (UTF-8, musl state machine)
// ============================================================

const SA: u8 = 0xC2;
const SB: u8 = 0xF4;

const BITTAB: [u32; 51] = [
    0xC000_0002, 0xC000_0003, 0xC000_0004, 0xC000_0005, 0xC000_0006, 0xC000_0007,
    0xC000_0008, 0xC000_0009, 0xC000_000A, 0xC000_000B, 0xC000_000C, 0xC000_000D,
    0xC000_000E, 0xC000_000F, 0xC000_0010, 0xC000_0011, 0xC000_0012, 0xC000_0013,
    0xC000_0014, 0xC000_0015, 0xC000_0016, 0xC000_0017, 0xC000_0018, 0xC000_0019,
    0xC000_001A, 0xC000_001B, 0xC000_001C, 0xC000_001D, 0xC000_001E, 0xC000_001F,
    0xB300_0000, 0xC300_0001, 0xC300_0002, 0xC300_0003, 0xC300_0004, 0xC300_0005,
    0xC300_0006, 0xC300_0007, 0xC300_0008, 0xC300_0009, 0xC300_000A, 0xC300_000B,
    0xC300_000C, 0xD300_000D, 0xC300_000E, 0xC300_000F, 0xBB0C_0000, 0xC30C_0001,
    0xC30C_0002, 0xC30C_0003, 0xDB0C_0004,
];

#[inline]
fn mb_oob(c: u32, b: u8) -> u32 {
    let b3 = (b >> 3) as u32;
    ((b3.wrapping_sub(0x10)) | (b3.wrapping_add((c as i32 >> 26) as u32))) & !7
}

static mut MB_STATE: c_uint = 0;

/// Select the caller's multibyte state or the process-global fallback used
/// when `st` is null.
#[inline]
unsafe fn mb_state_or(st: *mut c_uint) -> &'static mut c_uint {
    if st.is_null() { &mut *(&raw mut MB_STATE) } else { &mut *st }
}

const MB_CUR_MAX_VAL_UTF8: usize = 4;
const MB_CUR_MAX_VAL_C: usize = 1;

#[no_mangle]
pub extern "C" fn __ctype_get_mb_cur_max() -> usize {
    unsafe { if LOCALE_CTYPE_UTF8 { MB_CUR_MAX_VAL_UTF8 } else { MB_CUR_MAX_VAL_C } }
}

#[no_mangle]
pub unsafe extern "C" fn mbrtowc(
    wc: *mut c_int,
    src: *const c_char,
    n: usize,
    st: *mut c_uint,
) -> usize {
    let s = src as *const u8;
    let n0 = n;
    let mut dummy: c_int = 0;
    let wc = if wc.is_null() { &mut dummy } else { &mut *wc };
    let st = mb_state_or(st);

    let mut c: u32 = *st;
    let mut s = s;
    let mut n = n;

    if s.is_null() {
        if c != 0 { *st = 0; ERRNO!() = EILSEQ; return !0usize; }
        return 0;
    }

    if n == 0 { return !1usize; }

    if unsafe { !LOCALE_CTYPE_UTF8 } {
        let b = *s;
        *wc = c_locale_codeunit(b) as c_int;
        return if b == 0 { 0 } else { 1 };
    }

    if c == 0 {
        if *s < 0x80 {
            let r = if *s == 0 { 0 } else { 1 };
            *wc = *s as c_int;
            return r;
        }
        if (*s).wrapping_sub(SA) as u32 > (SB - SA) as u32 {
            *st = 0; ERRNO!() = EILSEQ; return !0usize;
        }
        c = BITTAB[*s as usize - SA as usize];
        s = s.add(1);
        n -= 1;
    }

    if n > 0 {
        if mb_oob(c, *s) != 0 {
            *st = 0; ERRNO!() = EILSEQ; return !0usize;
        }
        loop {
            c = (c << 6) | (*s as u32).wrapping_sub(0x80);
            s = s.add(1);
            n -= 1;
            if c & (1u32 << 31) == 0 {
                *st = 0;
                *wc = c as c_int;
                return n0 - n;
            }
            if n == 0 { break; }
            if (*s).wrapping_sub(0x80) >= 0x40 {
                *st = 0; ERRNO!() = EILSEQ; return !0usize;
            }
        }
    }

    *st = c;
    !1usize
}

#[no_mangle]
pub unsafe extern "C" fn wcrtomb(
    s: *mut c_char,
    wc: c_int,
    _st: *mut c_uint,
) -> usize {
    if s.is_null() { return 1; }
    let wc_u = wc as u32;
    if unsafe { !LOCALE_CTYPE_UTF8 } {
        if wc_u < 0x80 {
            *s = wc as c_char;
            return 1;
        }
        if c_locale_is_codeunit(wc_u) {
            *s = (wc_u & 0xFF) as c_char;
            return 1;
        }
        ERRNO!() = EILSEQ;
        return !0usize;
    }
    if wc_u < 0x80 {
        *s = wc as c_char;
        return 1;
    }
    if wc_u < 0x800 {
        *s = (0xC0 | (wc_u >> 6)) as c_char;
        *s.add(1) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 2;
    }
    if wc_u < UNICODE_SURROGATE_START || wc_u.wrapping_sub(UNICODE_SURROGATE_END_EXCL) < 0x2000 {
        *s = (0xE0 | (wc_u >> 12)) as c_char;
        *s.add(1) = (0x80 | ((wc_u >> 6) & 0x3F)) as c_char;
        *s.add(2) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 3;
    }
    if wc_u.wrapping_sub(UNICODE_SUPPLEMENTARY_BASE) < UNICODE_SUPPLEMENTARY_SIZE {
        *s = (0xF0 | (wc_u >> 18)) as c_char;
        *s.add(1) = (0x80 | ((wc_u >> 12) & 0x3F)) as c_char;
        *s.add(2) = (0x80 | ((wc_u >> 6) & 0x3F)) as c_char;
        *s.add(3) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 4;
    }
    ERRNO!() = EILSEQ;
    !0usize
}

#[no_mangle]
pub unsafe extern "C" fn mblen(s: *const c_char, n: usize) -> c_int {
    if s.is_null() { return 0; }
    mbtowc(core::ptr::null_mut(), s, n)
}

#[no_mangle]
pub unsafe extern "C" fn mbtowc(
    wc: *mut c_int,
    src: *const c_char,
    n: usize,
) -> c_int {
    if src.is_null() { return 0; }
    if n == 0 { ERRNO!() = EILSEQ; return -1; }
    let mut state: c_uint = 0;
    let mut w: c_int = 0;
    let r = mbrtowc(&mut w, src, n, &mut state);
    if r == !0usize { return -1; }
    if !wc.is_null() { *wc = w; }
    r as c_int
}

#[no_mangle]
pub unsafe extern "C" fn wctomb(s: *mut c_char, wc: c_int) -> c_int {
    if s.is_null() { return 0; }
    wcrtomb(s, wc, core::ptr::null_mut()) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn mbsrtowcs(
    ws: *mut c_int,
    src: *mut *const c_char,
    wn: usize,
    st: *mut c_uint,
) -> usize {
    let st = mb_state_or(st);
    let mut s = *src as *const u8;
    let wn0 = wn;
    let mut wn = wn;

    if ws.is_null() {
        while *s != 0 {
            if *s < 0x80 {
                s = s.add(1);
                wn = wn.wrapping_sub(1);
                continue;
            }
            let mut wc: c_int = 0;
            let r = mbrtowc(&mut wc, s as *const c_char, 4, st);
            if r == !0usize { return !0usize; }
            if r == !1usize { return wn0 - wn; }
            s = s.add(r);
            wn = wn.wrapping_sub(1);
        }
        wn0 - wn
    } else {
        let mut w = ws;
        while wn > 0 {
            if *s < 0x80 {
                let c = *s as c_int;
                *w = c;
                if c == 0 {
                    *src = core::ptr::null();
                    return wn0 - wn;
                }
                w = w.add(1);
                s = s.add(1);
                wn -= 1;
                continue;
            }
            let mut wc: c_int = 0;
            let r = mbrtowc(&mut wc, s as *const c_char, 4, st);
            if r == !0usize {
                *src = s as *const c_char;
                return !0usize;
            }
            if r == !1usize {
                *src = s as *const c_char;
                return wn0 - wn;
            }
            *w = wc;
            w = w.add(1);
            s = s.add(r);
            wn -= 1;
        }
        *src = s as *const c_char;
        wn0 - wn
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsrtombs(
    s: *mut c_char,
    ws: *mut *const c_int,
    n: usize,
    _st: *mut c_uint,
) -> usize {
    let mut w = *ws;
    let n0 = n;
    let mut n = n;

    if s.is_null() {
        let mut count = 0usize;
        while *w != 0 {
            if (*w as u32) < 0x80 {
                count += 1;
            } else {
                let mut buf = [0u8; 4];
                let r = wcrtomb(buf.as_mut_ptr() as *mut c_char, *w, core::ptr::null_mut());
                if r == !0usize { return !0usize; }
                count += r;
            }
            w = w.add(1);
        }
        return count;
    }

    let mut dst = s;
    while n > 0 {
        if (*w as u32) < 0x80 {
            *dst = *w as c_char;
            if *w == 0 {
                *ws = core::ptr::null();
                return n0 - n;
            }
            dst = dst.add(1);
            n -= 1;
        } else {
            let r = wcrtomb(dst, *w, core::ptr::null_mut());
            if r == !0usize {
                *ws = w;
                return !0usize;
            }
            if r > n {
                *ws = w;
                return n0 - n;
            }
            dst = dst.add(r);
            n -= r;
        }
        w = w.add(1);
    }
    *ws = w;
    n0
}

#[no_mangle]
pub unsafe extern "C" fn mbstowcs(
    ws: *mut c_int,
    src: *const c_char,
    wn: usize,
) -> usize {
    let mut src_ptr = src;
    mbsrtowcs(ws, &mut src_ptr, wn, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn wcstombs(
    s: *mut c_char,
    ws: *const c_int,
    n: usize,
) -> usize {
    let mut ws_ptr = ws;
    wcsrtombs(s, &mut ws_ptr, n, core::ptr::null_mut())
}

// ============================================================
// wchar: btowc / wctob / mbsinit / mbrlen
// ============================================================

// musl's C locale represents bytes >= 0x80 as the single-byte "code units"
// 0xDF80..0xDFFF so they are distinct from Unicode code points.
#[inline]
fn c_locale_codeunit(b: u8) -> u32 {
    0xdfff & (b as i8 as i32) as u32
}

#[inline]
fn c_locale_is_codeunit(c: u32) -> bool {
    c.wrapping_sub(0xDF80) < 0x80
}

#[no_mangle]
pub extern "C" fn btowc(c: c_int) -> wint_t {
    if c == -1 { return WEOF; }
    unsafe {
        if LOCALE_CTYPE_UTF8 {
            if (c as u32) < 128 { c as wint_t } else { WEOF }
        } else {
            c_locale_codeunit(c as u8) as wint_t
        }
    }
}

#[no_mangle]
pub extern "C" fn wctob(c: wint_t) -> c_int {
    unsafe {
        if LOCALE_CTYPE_UTF8 {
            if c < 128 { c as c_int } else { -1 }
        } else if c < 128 {
            c as c_int
        } else if c_locale_is_codeunit(c) {
            (c & 0xFF) as c_int
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn mbsinit(s: *const c_uint) -> c_int {
    if s.is_null() || *s == 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn mbrlen(s: *const c_char, n: usize, ps: *mut c_uint) -> usize {
    mbrtowc(core::ptr::null_mut(), s, n, ps)
}

