// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// stdlib: strtod / strtof / strtold
// ============================================================

// Correctly-rounded float parser.
// Decimal: collects digits into buffer, uses core::str::FromStr.
// Hex: u128 integer arithmetic with round-to-nearest-even.
// is_f32: true for strtof (f32::from_str), false for strtod/strtold (f64::from_str).
unsafe fn parse_float(s: *const u8, endptr: *mut *mut u8, is_f32: bool) -> f64 {
    let mut p = s;
    // skip whitespace
    while *p == b' ' || *p == b'\t' || *p == b'\n' || *p == b'\r' { p = p.add(1); }
    let mut neg = false;
    if *p == b'-' { neg = true; p = p.add(1); }
    else if *p == b'+' { p = p.add(1); }

    // inf
    if (*p | 0x20) == b'i'
        && (*p.add(1) | 0x20) == b'n'
        && (*p.add(2) | 0x20) == b'f'
    {
        p = p.add(3);
        // optionally consume "inity"
        if (*p | 0x20) == b'i'
            && (*p.add(1) | 0x20) == b't'
            && (*p.add(2) | 0x20) == b'y'
        {
            p = p.add(3);
        }
        if !endptr.is_null() { *endptr = p as *mut u8; }
        return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    // nan
    if (*p | 0x20) == b'n'
        && (*p.add(1) | 0x20) == b'a'
        && (*p.add(2) | 0x20) == b'n'
    {
        p = p.add(3);
        // optionally consume (n-char-sequence)
        if *p == b'(' {
            p = p.add(1);
            while *p != 0 && *p != b')' { p = p.add(1); }
            if *p == b')' { p = p.add(1); }
        }
        if !endptr.is_null() { *endptr = p as *mut u8; }
        return if neg { -f64::NAN } else { f64::NAN };
    }

    // hex float
    if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') {
        p = p.add(2);
        let mut mant: u128 = 0;
        let mut frac_hex: i64 = 0;
        let mut dropped: i64 = 0;
        let mut tail_nonzero = false;
        let mut found = false;
        let mut in_frac = false;

        loop {
            if let Some(d) = hex_val(*p) {
                if mant <= u128::MAX / 16 {
                    mant = mant * 16 + d as u128;
                } else {
                    // Too many significant digits for u128: keep the leading
                    // ones and remember that the dropped tail is nonzero.
                    dropped += 1;
                    if d != 0 { tail_nonzero = true; }
                }
                if in_frac { frac_hex += 1; }
                p = p.add(1);
                found = true;
            } else if *p == b'.' && !in_frac {
                in_frac = true;
                p = p.add(1);
            } else {
                break;
            }
        }

        if !found {
            if !endptr.is_null() { *endptr = s as *mut u8; }
            return 0.0;
        }

        // binary exponent p[+-]N
        let mut bin_exp: i32 = 0;
        if *p == b'p' || *p == b'P' {
            p = p.add(1);
            let mut exp_neg = false;
            if *p == b'-' { exp_neg = true; p = p.add(1); }
            else if *p == b'+' { p = p.add(1); }
            while *p >= b'0' && *p <= b'9' {
                if bin_exp < 100000 {
                    bin_exp = bin_exp * 10 + (*p - b'0') as i32;
                }
                p = p.add(1);
            }
            if exp_neg { bin_exp = -bin_exp; }
        }

        if !endptr.is_null() { *endptr = p as *mut u8; }
        if mant == 0 { return if neg { -0.0 } else { 0.0 }; }

        // The exponent must account for every fractional digit, including
        // the ones dropped from `mant`.
        let total_exp = bin_exp as i64 - 4 * (frac_hex - dropped);
        let r = hex_mant_to_f64(mant, total_exp, neg, tail_nonzero);
        note_float_range(r, true, is_f32);
        return r;
    }

    // decimal: collect the significant digits into a bounded buffer and let
    // Rust's parser do the correctly-rounded conversion. Keeping only the
    // leading digits (and folding the rest into the exponent) keeps the stack
    // footprint small without changing the result.
    const MAX_SIG_DIGITS: usize = 768;
    let mut buf = [0u8; MAX_SIG_DIGITS + 32];
    let mut n = 0usize;
    let mut found_digit = false;
    let mut nonzero = false;
    let mut frac_kept: i64 = 0;
    let mut frac_lead_zeros: i64 = 0;
    let mut int_dropped: i64 = 0;

    while *p >= b'0' && *p <= b'9' {
        if *p != b'0' { nonzero = true; }
        if nonzero {
            if n < MAX_SIG_DIGITS {
                buf[n] = *p;
                n += 1;
            } else {
                int_dropped += 1;
            }
        }
        p = p.add(1);
        found_digit = true;
    }
    if *p == b'.' {
        p = p.add(1);
        while *p >= b'0' && *p <= b'9' {
            if *p != b'0' { nonzero = true; }
            if !nonzero {
                // Leading fractional zeros still shift the exponent.
                frac_lead_zeros += 1;
            } else if n < MAX_SIG_DIGITS {
                buf[n] = *p;
                n += 1;
                frac_kept += 1;
            }
            p = p.add(1);
            found_digit = true;
        }
    }
    if !found_digit {
        ERRNO!() = EINVAL;
        if !endptr.is_null() { *endptr = s as *mut u8; }
        return 0.0;
    }
    let mut exp10: i64 = int_dropped - frac_kept - frac_lead_zeros;
    if *p == b'e' || *p == b'E' {
        let mut q = p.add(1);
        let mut exp_neg = false;
        if *q == b'-' { exp_neg = true; q = q.add(1); }
        else if *q == b'+' { q = q.add(1); }
        // Without at least one exponent digit the 'e' is not part of the
        // number; leave the end pointer on it.
        if *q >= b'0' && *q <= b'9' {
            let mut e: i64 = 0;
            while *q >= b'0' && *q <= b'9' {
                e = e.saturating_mul(10).saturating_add((*q - b'0') as i64);
                if e > 1_000_000 { e = 1_000_000; }
                q = q.add(1);
            }
            exp10 += if exp_neg { -e } else { e };
            p = q;
        }
    }
    if !endptr.is_null() { *endptr = p as *mut u8; }
    if n == 0 {
        // Every digit was zero.
        return if neg { -0.0 } else { 0.0 };
    }

    // Represent the kept digits as an integer times a power of ten.
    buf[n] = b'e';
    n += 1;
    let mut tmp = [0u8; 24];
    let mut tn = 0usize;
    let e_abs = exp10.unsigned_abs();
    if e_abs == 0 {
        tmp[0] = b'0';
        tn = 1;
    } else {
        let mut v = e_abs;
        while v > 0 && tn < tmp.len() {
            tmp[tn] = b'0' + (v % 10) as u8;
            v /= 10;
            tn += 1;
        }
    }
    if exp10 < 0 { buf[n] = b'-'; n += 1; }
    for k in (0..tn).rev() {
        buf[n] = tmp[k];
        n += 1;
    }

    let s_str = core::str::from_utf8_unchecked(
        core::slice::from_raw_parts(buf.as_ptr(), n),
    );
    if is_f32 {
        match <f32 as core::str::FromStr>::from_str(s_str) {
            Ok(v) => {
                let r = if neg { -(v as f64) } else { v as f64 };
                note_float_range(r, nonzero, true);
                r
            }
            Err(_) => { if !endptr.is_null() { *endptr = s as *mut u8; } 0.0 }
        }
    } else {
        match <f64 as core::str::FromStr>::from_str(s_str) {
            Ok(v) => {
                let r = if neg { -v } else { v };
                note_float_range(r, nonzero, false);
                r
            }
            Err(_) => { if !endptr.is_null() { *endptr = s as *mut u8; } 0.0 }
        }
    }
}

/// Set ERANGE for overflow and underflow, like musl's floatscan.
unsafe fn note_float_range(r: f64, nonzero: bool, is_f32: bool) {
    let (infinite, zero, subnormal) = if is_f32 {
        let v = r as f32;
        (v.is_infinite(), v == 0.0, v != 0.0 && v.abs() < f32::MIN_POSITIVE)
    } else {
        (r.is_infinite(), r == 0.0, r != 0.0 && r.abs() < f64::MIN_POSITIVE)
    };
    if infinite || subnormal || (zero && nonzero) {
        ERRNO!() = ERANGE;
    }
}

// Convert hex mantissa integer to f64 with correct rounding.
// mant: integer from parsed hex digits. total_exp: bin_exp - 4 * frac_hex_digits.
// tail_nonzero records dropped digits below `mant` for the sticky bit.
// Value = mant * 2^total_exp.
fn hex_mant_to_f64(mant: u128, total_exp: i64, neg: bool, tail_nonzero: bool) -> f64 {
    if mant == 0 { return if neg { -0.0 } else { 0.0 }; }

    let msb = (127 - mant.leading_zeros()) as i64;
    let unbiased_exp = msb + total_exp;
    let biased_exp = unbiased_exp + 1023;

    // Overflow → infinity
    if biased_exp >= 2047 {
        return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
    }

    // Subnormal or underflow
    if biased_exp <= 0 {
        // subnormal: biased_exp=0, effective exponent=-1022
        // mantissa = round(mant * 2^(total_exp + 1074))
        let shift = total_exp + 1074;
        if shift < 0 {
            return if neg { -0.0 } else { 0.0 };
        }
        if shift > 128 {
            return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
        }
        let shifted = mant << (shift as u32);
        let mantissa52 = (shifted as u64) & 0x000FFFFFFFFFFFFF;
        let guard = ((shifted >> 52) & 1) != 0;
        let sticky = (shifted >> 53) != 0 || tail_nonzero;
        let mut result = mantissa52;
        if guard && (sticky || (result & 1) != 0) {
            result += 1;
            if result > 0x000FFFFFFFFFFFFF {
                // carry → smallest normal
                return if neg { -f64::from_bits(1u64 << 52) } else { f64::from_bits(1u64 << 52) };
            }
        }
        return if neg { -f64::from_bits(result) } else { f64::from_bits(result) };
    }

    // Normal: biased_exp in [1, 2046]
    let shift = msb - 52;

    if shift <= 0 {
        // fewer than 53 bits, no rounding needed
        let mantissa = ((mant as u64) & ((1u64 << msb as u32) - 1)) << ((-shift) as u32);
        let bits = ((biased_exp as u64) << 52) | mantissa;
        return if neg { -f64::from_bits(bits) } else { f64::from_bits(bits) };
    }

    // shift > 0: round to nearest even
    let mantissa = ((mant >> shift as u32) as u64) & 0x000FFFFFFFFFFFFF;
    let guard = ((mant >> (shift as u32 - 1)) & 1) != 0;
    let sticky = (if shift > 1 {
        (mant & ((1u128 << (shift as u32 - 1)) - 1)) != 0
    } else { false }) || tail_nonzero;

    let mut result_mant = mantissa;
    let mut result_exp = biased_exp;
    if guard && (sticky || (result_mant & 1) != 0) {
        result_mant += 1;
        if result_mant > 0x000FFFFFFFFFFFFF {
            result_mant = 0;
            result_exp += 1;
            if result_exp >= 2047 {
                return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
            }
        }
    }

    let bits = ((result_exp as u64) << 52) | result_mant;
    if neg { -f64::from_bits(bits) } else { f64::from_bits(bits) }
}

unsafe fn hex_val(c: u8) -> Option<u8> {
    if c >= b'0' && c <= b'9' { Some(c - b'0') }
    else if c >= b'a' && c <= b'f' { Some(c - b'a' + 10) }
    else if c >= b'A' && c <= b'F' { Some(c - b'A' + 10) }
    else { None }
}

#[no_mangle]
pub unsafe extern "C" fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    let mut end: *mut u8 = s as *mut u8;
    let r = parse_float(s as *const u8, &mut end, false);
    if !endptr.is_null() { *endptr = end as *mut c_char; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32 {
    let mut end: *mut u8 = s as *mut u8;
    let r = parse_float(s as *const u8, &mut end, true);
    if !endptr.is_null() { *endptr = end as *mut c_char; }
    r as f32
}

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    strtod(s, endptr)
}

#[no_mangle]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f128 {
    f64_to_f128(strtod(s, endptr))
}

#[no_mangle]
#[cfg(any(target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f128 {
    f64_to_f128(strtod(s, endptr))
}

#[no_mangle]
pub unsafe extern "C" fn atof(s: *const c_char) -> f64 {
    strtod(s, core::ptr::null_mut())
}

