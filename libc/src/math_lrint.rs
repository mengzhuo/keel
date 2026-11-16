// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Port of musl lrint/llrint/lrintf/llrintf/lrintl/llrintl
//
// On x86_64: c_long = i64 (LONG_MAX >= 2^53), c_longlong = i64, long double = f64.
// All six functions reduce to: round-to-integer-using-current-FP-mode, then cast.
//
// The rint/rintf helpers use musl's `x + toint - toint` trick which rounds via
// the current FP rounding mode (MXCSR on x86_64), unlike Rust's `as` which
// truncates toward zero.

// musl rint: rounds f64 to integer using current FP rounding mode
#[no_mangle]
pub extern "C" fn rint(x: f64) -> f64 {
    let u = asuint64(x);
    let e = ((u >> 52) & 0x7ff) as i32;
    let s = u >> 63;

    // Already an integer (exponent >= 52 means no fractional bits)
    if e >= 0x3ff + 52 {
        return x;
    }

    // toint = 1/DBL_EPSILON = 2^52 = 4503599627370496.0
    let toint = 4503599627370496.0f64;
    let y = if s != 0 {
        x - toint + toint
    } else {
        x + toint - toint
    };

    if y == 0.0 {
        if s != 0 { -0.0 } else { 0.0 }
    } else {
        y
    }
}

// musl rintf: rounds f32 to integer using current FP rounding mode
#[no_mangle]
pub extern "C" fn rintf(x: f32) -> f32 {
    let u = asuint(x);
    let e = ((u >> 23) & 0xff) as i32;
    let s = u >> 31;

    // Already an integer (exponent >= 23 means no fractional bits)
    if e >= 0x7f + 23 {
        return x;
    }

    // toint = 1/FLT_EPSILON = 2^23 = 8388608.0
    let toint = 8388608.0f32;
    let y = if s != 0 {
        x - toint + toint
    } else {
        x + toint - toint
    };

    if y == 0.0 {
        if s != 0 { -0.0f32 } else { 0.0f32 }
    } else {
        y
    }
}

// On x86_64 with c_long = i64 (LONG_MAX >= 2^53), the musl slow path
// (lrint_slow with FE_INEXACT handling) is not needed. Simple cast suffices
// because: (1) rint returns an exact integer as f64, (2) the largest integer
// representable as f64 is 2^53 which fits in i64, (3) for |x| >= 2^52 rint
// returns x unchanged and the saturating `as` cast handles overflow.

const LRINT_TWO63: f64 = 9223372036854775808.0;

/// Round with the current FP mode and convert to i64.  Rust's saturating cast
/// does not raise FE_INVALID, so check the representable range explicitly.
#[inline]
unsafe fn lrint_i64(x: f64) -> i64 {
    let r = rint(x);
    if r.is_nan() || r >= LRINT_TWO63 || r < -LRINT_TWO63 {
        feraiseexcept(FE_INVALID);
        return i64::MIN;
    }
    r as i64
}

#[inline]
unsafe fn lrintf_i64(x: f32) -> i64 {
    lrint_i64(rintf(x) as f64)
}

#[no_mangle]
pub extern "C" fn lrint(x: f64) -> c_long {
    unsafe { lrint_i64(x) }
}

#[no_mangle]
pub extern "C" fn llrint(x: f64) -> c_longlong {
    unsafe { lrint_i64(x) }
}

#[no_mangle]
pub extern "C" fn lrintf(x: f32) -> c_long {
    unsafe { lrintf_i64(x) }
}

#[no_mangle]
pub extern "C" fn llrintf(x: f32) -> c_longlong {
    unsafe { lrintf_i64(x) }
}

// On x86_64 with -mlong-double-64: long double = f64, delegate to lrint/llrint.
// On aarch64: long double = f128 (IEEE quad), cast to f64 then round.
#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "C" fn lrintl(x: f64) -> c_long {
    lrint(x)
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub extern "C" fn llrintl(x: f64) -> c_longlong {
    llrint(x)
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn lrintl(x: f128) -> c_long {
    lrint(f128_to_f64(x))
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn llrintl(x: f128) -> c_longlong {
    llrint(f128_to_f64(x))
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub extern "C" fn lrintl(x: f128) -> c_long {
    lrint(f128_to_f64(x))
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub extern "C" fn llrintl(x: f128) -> c_longlong {
    llrint(f128_to_f64(x))
}
