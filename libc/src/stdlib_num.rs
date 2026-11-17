// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// stdlib.h: integer conversion, abs, rand
// ============================================================

unsafe fn parse_digit(c: u8, base: c_int) -> Option<u8> {
    if c == 0 { return None; }
    let d = if c >= b'0' && c <= b'9' {
        c - b'0'
    } else if c >= b'a' && c <= b'z' {
        c - b'a' + 10
    } else if c >= b'A' && c <= b'Z' {
        c - b'A' + 10
    } else {
        return None;
    };
    if (d as c_int) < base {
        Some(d)
    } else {
        None
    }
}

unsafe fn parse_prefix(s: *const u8, base: *mut c_int) -> *const u8 {
    let mut p = s;
    if *base == 0 {
        if *p == b'0' {
            if (*p.add(1) == b'x' || *p.add(1) == b'X') && parse_digit(*p.add(2), 16).is_some() {
                *base = 16;
                p = p.add(2);
            } else {
                *base = 8;
            }
        } else {
            *base = 10;
        }
    } else if *base == 16 {
        if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') && parse_digit(*p.add(2), 16).is_some() {
            p = p.add(2);
        }
    }
    p
}

unsafe fn strtox(
    s: *const c_char,
    endptr: *mut *mut c_char,
    mut base: c_int,
    pos_limit: u64,
    neg_limit: u64,
) -> (u64, bool, bool) {
    let s0 = s as *const u8;
    let mut p = s0;
    while isspace(*p as c_int) != 0 {
        p = p.add(1);
    }

    if base < 0 || base == 1 || base > 36 {
        if !endptr.is_null() {
            *endptr = s as *mut c_char;
        }
        ERRNO!() = EINVAL;
        return (0, false, false);
    }

    let mut neg = false;
    match *p {
        b'-' => { neg = true; p = p.add(1); }
        b'+' => p = p.add(1),
        _ => {}
    }

    p = parse_prefix(p, &mut base);

    let limit = if neg { neg_limit } else { pos_limit };
    let mut val: u64 = 0;
    let mut any = false;
    while let Some(d) = parse_digit(*p, base) {
        any = true;
        if val > (limit - d as u64) / base as u64 {
            while parse_digit(*p, base).is_some() { p = p.add(1); }
            if !endptr.is_null() { *endptr = p as *mut c_char; }
            ERRNO!() = ERANGE;
            return (limit, true, neg);
        }
        val = val * base as u64 + d as u64;
        p = p.add(1);
    }

    if !endptr.is_null() {
        *endptr = if any { p as *mut c_char } else { s as *mut c_char };
    }
    (val, false, neg)
}

#[no_mangle]
pub unsafe extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long {
    let pos_limit = c_long::MAX as u64;
    let neg_limit = pos_limit.wrapping_add(1);
    let (val, overflow, neg) = strtox(s, endptr, base, pos_limit, neg_limit);
    if overflow {
        return if neg { c_long::MIN } else { c_long::MAX };
    }
    if neg { val.wrapping_neg() as c_long } else { val as c_long }
}

#[no_mangle]
pub unsafe extern "C" fn strtoul(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong {
    let limit = c_ulong::MAX as u64;
    let (val, overflow, neg) = strtox(s, endptr, base, limit, limit);
    if overflow {
        return c_ulong::MAX;
    }
    if neg { val.wrapping_neg() as c_ulong } else { val as c_ulong }
}

#[no_mangle]
pub unsafe extern "C" fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_longlong {
    let pos_limit = c_longlong::MAX as u64;
    let neg_limit = pos_limit.wrapping_add(1);
    let (val, overflow, neg) = strtox(s, endptr, base, pos_limit, neg_limit);
    if overflow {
        return if neg { c_longlong::MIN } else { c_longlong::MAX };
    }
    if neg { val.wrapping_neg() as c_longlong } else { val as c_longlong }
}

#[no_mangle]
pub unsafe extern "C" fn strtoull(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulonglong {
    let limit = c_ulonglong::MAX as u64;
    let (val, overflow, neg) = strtox(s, endptr, base, limit, limit);
    if overflow {
        return c_ulonglong::MAX;
    }
    if neg { val.wrapping_neg() as c_ulonglong } else { val as c_ulonglong }
}

#[no_mangle]
pub unsafe extern "C" fn atoi(s: *const c_char) -> c_int {
    strtol(s, core::ptr::null_mut(), 10) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn atol(s: *const c_char) -> c_long {
    strtol(s, core::ptr::null_mut(), 10)
}

#[no_mangle]
pub unsafe extern "C" fn atoll(s: *const c_char) -> c_longlong {
    strtoll(s, core::ptr::null_mut(), 10)
}

#[no_mangle]
pub extern "C" fn abs(n: c_int) -> c_int {
    if n < 0 {
        -n
    } else {
        n
    }
}

#[no_mangle]
pub extern "C" fn labs(n: c_long) -> c_long {
    if n < 0 {
        -n
    } else {
        n
    }
}

#[no_mangle]
pub extern "C" fn llabs(n: c_longlong) -> c_longlong {
    if n < 0 {
        -n
    } else {
        n
    }
}

static mut RAND_STATE: c_uint = 1;

#[no_mangle]
pub extern "C" fn srand(seed: c_uint) {
    unsafe {
        RAND_STATE = seed;
    }
}

#[no_mangle]
pub extern "C" fn rand() -> c_int {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        ((RAND_STATE >> 16) & 0x7fff) as c_int
    }
}

// ============================================================
// stdlib.h: div, ldiv, lldiv
// ============================================================

#[repr(C)]
pub struct div_t { pub quot: c_int, pub rem: c_int }
#[repr(C)]
pub struct ldiv_t { pub quot: c_long, pub rem: c_long }
#[repr(C)]
pub struct lldiv_t { pub quot: c_longlong, pub rem: c_longlong }

#[no_mangle]
pub extern "C" fn div(num: c_int, den: c_int) -> div_t {
    div_t { quot: num / den, rem: num % den }
}

#[no_mangle]
pub extern "C" fn ldiv(num: c_long, den: c_long) -> ldiv_t {
    ldiv_t { quot: num / den, rem: num % den }
}

#[no_mangle]
pub extern "C" fn lldiv(num: c_longlong, den: c_longlong) -> lldiv_t {
    lldiv_t { quot: num / den, rem: num % den }
}

// ============================================================
// bsearch
// ============================================================

type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

#[no_mangle]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nel: usize,
    width: usize,
    cmp: CmpFn,
) -> *mut c_void {
    let mut base = base as *const u8;
    let mut nel = nel;
    while nel > 0 {
        let trial = base.add(width * (nel / 2));
        let sign = cmp(key, trial as *const c_void);
        if sign < 0 {
            nel /= 2;
        } else if sign > 0 {
            base = trial.add(width);
            nel -= nel / 2 + 1;
        } else {
            return trial as *mut c_void;
        }
    }
    null_mut()
}

include!("signal.rs");
include!("process.rs");
