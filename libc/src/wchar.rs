// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// wchar: wide string functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn wcslen(s: *const wchar_t) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 { len += 1; }
    len
}

#[no_mangle]
pub unsafe extern "C" fn wcschr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let ch = *s.add(i);
        if ch == c { return s.add(i) as *mut wchar_t; }
        if ch == 0 { return null_mut(); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsrchr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t {
    let mut i = 0;
    let mut last: *mut wchar_t = null_mut();
    loop {
        let ch = *s.add(i);
        if ch == c { last = s.add(i) as *mut wchar_t; }
        if ch == 0 { return last; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsstr(haystack: *const wchar_t, needle: *const wchar_t) -> *mut wchar_t {
    if *needle == 0 { return haystack as *mut wchar_t; }
    let mut i = 0;
    loop {
        let h = *haystack.add(i);
        if h == 0 { return null_mut(); }
        if h == *needle {
            let mut j = 0;
            loop {
                let n = *needle.add(j);
                if n == 0 { return haystack.add(i) as *mut wchar_t; }
                if *haystack.add(i + j) != n { break; }
                j += 1;
            }
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscmp(s1: *const wchar_t, s2: *const wchar_t) -> c_int {
    let mut i = 0;
    loop {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b { return a - b; }
        if a == 0 { return 0; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsncmp(s1: *const wchar_t, s2: *const wchar_t, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b { return a - b; }
        if a == 0 { return 0; }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn wcscpy(dst: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 { break; }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsncpy(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t {
    let mut i = 0;
    while i < n && *src.add(i) != 0 { *dst.add(i) = *src.add(i); i += 1; }
    while i < n { *dst.add(i) = 0; i += 1; }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcscat(dst: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    while *dst.add(i) != 0 { i += 1; }
    let mut j = 0;
    loop {
        let c = *src.add(j);
        *dst.add(i + j) = c;
        if c == 0 { break; }
        j += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsncat(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t {
    let mut i = 0;
    while *dst.add(i) != 0 { i += 1; }
    let mut j = 0;
    while j < n && *src.add(j) != 0 { *dst.add(i + j) = *src.add(j); j += 1; }
    *dst.add(i + j) = 0;
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsdup(s: *const wchar_t) -> *mut wchar_t {
    let len = wcslen(s);
    let p = malloc((len + 1) * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
    if p.is_null() { return null_mut(); }
    core::ptr::copy_nonoverlapping(s, p, len + 1);
    p
}

#[no_mangle]
pub unsafe extern "C" fn wcsnlen(s: *const wchar_t, maxlen: usize) -> usize {
    let mut i = 0;
    while i < maxlen && *s.add(i) != 0 { i += 1; }
    i
}

#[no_mangle]
pub unsafe extern "C" fn wcsspn(s: *const wchar_t, accept: *const wchar_t) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return i; }
        let mut found = false;
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 { break; }
            if c == a { found = true; break; }
            j += 1;
        }
        if !found { return i; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscspn(s: *const wchar_t, reject: *const wchar_t) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return i; }
        let mut j = 0;
        loop {
            let r = *reject.add(j);
            if r == 0 { break; }
            if c == r { return i; }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcspbrk(s: *const wchar_t, accept: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return null_mut(); }
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 { break; }
            if c == a { return s.add(i) as *mut wchar_t; }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsxfrm(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> usize {
    let len = wcslen(src);
    if !dst.is_null() {
        let copy = if len < n { len } else { if n > 0 { n - 1 } else { 0 } };
        for i in 0..copy { *dst.add(i) = *src.add(i); }
        if n > 0 { *dst.add(copy) = 0; }
    }
    len
}

// ============================================================
// wchar: wcsto* number conversions
// ============================================================

unsafe fn wcs_to_bytes(ws: *const wchar_t, buf: *mut u8, bufsz: usize) -> usize {
    if bufsz == 0 {
        return 0;
    }
    let mut i = 0;
    while i < bufsz - 1 {
        let c = *ws.add(i);
        if c == 0 { break; }
        if (c as u32) > 127 { break; }
        *buf.add(i) = c as u8;
        i += 1;
    }
    *buf.add(i) = 0;
    i
}

macro_rules! wcsto_int {
    ($name:ident, $ret:ty, $conv:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            s: *const wchar_t,
            endptr: *mut *mut wchar_t,
            base: c_int,
        ) -> $ret {
            let cap = wcslen(s).saturating_add(1);
            let heap = cap > 256;
            let mut stack = [0u8; 256];
            let buf = if heap {
                crate::malloc(cap) as *mut u8
            } else {
                stack.as_mut_ptr()
            };
            if buf.is_null() {
                ERRNO!() = ENOMEM;
                return 0;
            }
            wcs_to_bytes(s, buf, cap);
            let mut end: *mut c_char = core::ptr::null_mut();
            let r = $conv(buf as *const c_char, &mut end, base);
            if !endptr.is_null() {
                let offset = end as usize - buf as usize;
                *endptr = s.add(offset) as *mut wchar_t;
            }
            if heap {
                crate::free(buf as *mut c_void);
            }
            r
        }
    };
}

wcsto_int!(wcstol, c_long, strtol);
wcsto_int!(wcstoul, c_ulong, strtoul);
wcsto_int!(wcstoll, c_longlong, strtoll);
wcsto_int!(wcstoull, c_ulonglong, strtoull);

#[no_mangle]
pub unsafe extern "C" fn wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    let cap = wcslen(s).saturating_add(1);
    let heap = cap > 256;
    let mut stack = [0u8; 256];
    let buf = if heap {
        crate::malloc(cap) as *mut u8
    } else {
        stack.as_mut_ptr()
    };
    if buf.is_null() {
        ERRNO!() = ENOMEM;
        return 0.0;
    }
    wcs_to_bytes(s, buf, cap);
    let mut end: *mut c_char = core::ptr::null_mut();
    let r = strtod(buf as *const c_char, &mut end);
    if !endptr.is_null() {
        let offset = end as usize - buf as usize;
        *endptr = s.add(offset) as *mut wchar_t;
    }
    if heap {
        crate::free(buf as *mut c_void);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32 { wcstod(s, endptr) as f32 }

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 { wcstod(s, endptr) }

#[no_mangle]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f128 { f64_to_f128(wcstod(s, endptr)) }

#[no_mangle]
#[cfg(any(target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f128 { f64_to_f128(wcstod(s, endptr)) }

#[no_mangle]
pub unsafe extern "C" fn wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> c_longlong { wcstoll(s, endptr, base) }

#[no_mangle]
pub unsafe extern "C" fn wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> c_ulonglong { wcstoull(s, endptr, base) }

// ============================================================
// wctype: classification functions
// ============================================================

const WC_ALPHA: c_int = 1;
const WC_DIGIT: c_int = 2;
const WC_SPACE: c_int = 4;
const WC_UPPER: c_int = 8;
const WC_LOWER: c_int = 16;
const WC_ALNUM: c_int = WC_ALPHA | WC_DIGIT;
const WC_BLANK: c_int = 32;
const WC_CNTRL: c_int = 64;
const WC_PUNCT: c_int = 128;
const WC_GRAPH: c_int = WC_ALNUM | WC_PUNCT;
const WC_PRINT: c_int = WC_GRAPH | WC_BLANK;
const WC_XDIGIT: c_int = 256;

#[inline]
fn wctable_bit(table: &[u8], wc: wint_t) -> bool {
    let wc = wc as u32;
    let page = table[(wc >> 8) as usize] as usize;
    (table[page * 32 + ((wc & 255) >> 3) as usize] >> (wc & 7)) & 1 != 0
}

/// Port of musl's iswalpha: table lookup below U+20000, then a broad
/// alphabetic range up to U+2FFFD.
#[no_mangle]
pub extern "C" fn iswalpha(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0x20000 {
        return wctable_bit(&WCTYPE_ALPHA_TABLE, c) as c_int;
    }
    if wc < 0x2fffe { 1 } else { 0 }
}

/// Port of musl's iswpunct.
#[no_mangle]
pub extern "C" fn iswpunct(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0x20000 {
        wctable_bit(&WCTYPE_PUNCT_TABLE, c) as c_int
    } else {
        0
    }
}

/// Port of musl's iswprint.
#[no_mangle]
pub extern "C" fn iswprint(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0xff {
        return (((wc + 1) & 0x7f) >= 0x21) as c_int;
    }
    if wc < 0x2028
        || wc.wrapping_sub(0x202a) < 0xd800 - 0x202a
        || wc.wrapping_sub(0xe000) < 0xfff9 - 0xe000
    {
        return 1;
    }
    if wc.wrapping_sub(0xfffc) > 0x10ffff - 0xfffc || (wc & 0xfffe) == 0xfffe {
        return 0;
    }
    1
}

/// Port of musl's iswspace: Unicode White_Space minus U+00A0/U+2007/U+202F,
/// plus U+1680 and U+180E are excluded (they have non-blank glyphs).
#[no_mangle]
pub extern "C" fn iswspace(c: wint_t) -> c_int {
    matches!(
        c as u32,
        0x20 | 0x09 | 0x0a | 0x0d | 0x0b | 0x0c | 0x85
            | 0x2000..=0x2006 | 0x2008..=0x200a
            | 0x2028 | 0x2029 | 0x205f | 0x3000
    ) as c_int
}

/// Port of musl's iswcntrl.
#[no_mangle]
pub extern "C" fn iswcntrl(c: wint_t) -> c_int {
    let wc = c as u32;
    ((wc < 32)
        || wc.wrapping_sub(0x7f) < 33
        || wc.wrapping_sub(0x2028) < 2
        || wc.wrapping_sub(0xfff9) < 3) as c_int
}

#[no_mangle]
pub extern "C" fn iswdigit(c: wint_t) -> c_int {
    ((c as u32).wrapping_sub(b'0' as u32) < 10) as c_int
}

#[no_mangle]
pub extern "C" fn iswxdigit(c: wint_t) -> c_int {
    let wc = c as u32;
    ((wc.wrapping_sub(b'0' as u32) < 10) || ((wc | 32).wrapping_sub(b'a' as u32) < 6)) as c_int
}

#[no_mangle]
pub extern "C" fn iswalnum(c: wint_t) -> c_int {
    (iswdigit(c) != 0 || iswalpha(c) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn iswblank(c: wint_t) -> c_int {
    (c == b' ' as wint_t || c == b'\t' as wint_t) as c_int
}

#[no_mangle]
pub extern "C" fn iswgraph(c: wint_t) -> c_int {
    (iswspace(c) == 0 && iswprint(c) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn iswlower(c: wint_t) -> c_int {
    (unsafe { towupper(c) } != c) as c_int
}

#[no_mangle]
pub extern "C" fn iswupper(c: wint_t) -> c_int {
    (unsafe { towlower(c) } != c) as c_int
}

static WCTYPE_ALNUM: c_int = WC_ALNUM;
static WCTYPE_ALPHA: c_int = WC_ALPHA;
static WCTYPE_BLANK: c_int = WC_BLANK;
static WCTYPE_CNTRL: c_int = WC_CNTRL;
static WCTYPE_DIGIT: c_int = WC_DIGIT;
static WCTYPE_GRAPH: c_int = WC_GRAPH;
static WCTYPE_LOWER: c_int = WC_LOWER;
static WCTYPE_PRINT: c_int = WC_PRINT;
static WCTYPE_PUNCT: c_int = WC_PUNCT;
static WCTYPE_SPACE: c_int = WC_SPACE;
static WCTYPE_UPPER: c_int = WC_UPPER;
static WCTYPE_XDIGIT: c_int = WC_XDIGIT;

#[no_mangle]
pub unsafe extern "C" fn wctype(name: *const c_char) -> wctype_t {
    if name.is_null() { return core::ptr::null(); }
    let n = name as *const u8;
    if crate::cstr_eq(n, b"alnum") { return &WCTYPE_ALNUM; }
    if crate::cstr_eq(n, b"alpha") { return &WCTYPE_ALPHA; }
    if crate::cstr_eq(n, b"blank") { return &WCTYPE_BLANK; }
    if crate::cstr_eq(n, b"cntrl") { return &WCTYPE_CNTRL; }
    if crate::cstr_eq(n, b"digit") { return &WCTYPE_DIGIT; }
    if crate::cstr_eq(n, b"graph") { return &WCTYPE_GRAPH; }
    if crate::cstr_eq(n, b"lower") { return &WCTYPE_LOWER; }
    if crate::cstr_eq(n, b"print") { return &WCTYPE_PRINT; }
    if crate::cstr_eq(n, b"punct") { return &WCTYPE_PUNCT; }
    if crate::cstr_eq(n, b"space") { return &WCTYPE_SPACE; }
    if crate::cstr_eq(n, b"upper") { return &WCTYPE_UPPER; }
    if crate::cstr_eq(n, b"xdigit") { return &WCTYPE_XDIGIT; }
    core::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn iswctype(c: wint_t, desc: wctype_t) -> c_int {
    if desc.is_null() { return 0; }
    if desc == core::ptr::addr_of!(WCTYPE_ALNUM) { return iswalnum(c); }
    if desc == core::ptr::addr_of!(WCTYPE_ALPHA) { return iswalpha(c); }
    if desc == core::ptr::addr_of!(WCTYPE_BLANK) { return iswblank(c); }
    if desc == core::ptr::addr_of!(WCTYPE_CNTRL) { return iswcntrl(c); }
    if desc == core::ptr::addr_of!(WCTYPE_DIGIT) { return iswdigit(c); }
    if desc == core::ptr::addr_of!(WCTYPE_GRAPH) { return iswgraph(c); }
    if desc == core::ptr::addr_of!(WCTYPE_LOWER) { return iswlower(c); }
    if desc == core::ptr::addr_of!(WCTYPE_PRINT) { return iswprint(c); }
    if desc == core::ptr::addr_of!(WCTYPE_PUNCT) { return iswpunct(c); }
    if desc == core::ptr::addr_of!(WCTYPE_SPACE) { return iswspace(c); }
    if desc == core::ptr::addr_of!(WCTYPE_UPPER) { return iswupper(c); }
    if desc == core::ptr::addr_of!(WCTYPE_XDIGIT) { return iswxdigit(c); }
    0
}

static WCTRANS_LOWER: c_int = 1;
static WCTRANS_UPPER: c_int = 2;

/// Port of musl's casemap() from ctype/towctrans.c.
fn casemap(c: wint_t, dir: i32) -> wint_t {
    let c0 = c;
    if c0 >= 0x20000 {
        return c0;
    }
    let b = (c0 >> 8) as usize;
    let cc = c0 & 255;
    let x = (cc / 3) as usize;
    let y = (cc % 3) as usize;
    const MT: [u32; 3] = [2048, 342, 57];
    let v = WCTYPE_CASEMAP_TAB[WCTYPE_CASEMAP_TAB[b] as usize * 86 + x] as u32;
    let v = (v * MT[y] >> 11) % 6;
    let r = WCTYPE_CASEMAP_RULES[(WCTYPE_CASEMAP_RULEBASES[b] + v as u8) as usize];
    let rt = (r & 255) as i32;
    let rd = r >> 8;
    if rt < 2 {
        return (c0 as i32 + (rd & -((rt) ^ dir))) as wint_t;
    }
    let mut xn = (rd & 0xff) as u32;
    let mut xb = (rd as u32) >> 8;
    while xn != 0 {
        let idx = (xb + xn / 2) as usize;
        let try_ = WCTYPE_CASEMAP_EXCEPTIONS[idx][0] as u32;
        if try_ == cc {
            let r2 = WCTYPE_CASEMAP_RULES[WCTYPE_CASEMAP_EXCEPTIONS[idx][1] as usize];
            let rt2 = (r2 & 255) as i32;
            let rd2 = r2 >> 8;
            if rt2 < 2 {
                return (c0 as i32 + (rd2 & -((rt2) ^ dir))) as wint_t;
            }
            return (c0 as i32 + if dir != 0 { -1 } else { 1 }) as wint_t;
        } else if try_ > cc {
            xn /= 2;
        } else {
            xb += xn / 2;
            xn -= xn / 2;
        }
    }
    c0
}

#[no_mangle]
pub unsafe extern "C" fn towlower(c: wint_t) -> wint_t {
    casemap(c, 0)
}

#[no_mangle]
pub unsafe extern "C" fn towupper(c: wint_t) -> wint_t {
    casemap(c, 1)
}

#[no_mangle]
pub unsafe extern "C" fn wctrans(name: *const c_char) -> wctrans_t {
    if name.is_null() { return core::ptr::null(); }
    let n = name as *const u8;
    if crate::cstr_eq(n, b"tolower") { return &WCTRANS_LOWER; }
    if crate::cstr_eq(n, b"toupper") { return &WCTRANS_UPPER; }
    core::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn towctrans(c: wint_t, desc: wctrans_t) -> wint_t {
    if desc.is_null() { return c; }
    match *desc {
        1 => towlower(c),
        2 => towupper(c),
        _ => c,
    }
}

