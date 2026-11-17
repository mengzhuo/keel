// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// exp10 functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn exp10(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn exp10f(x: f32) -> f32 {
    libm::powf(10.0, x)
}

// ============================================================
// fpclassify functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn finite(x: f64) -> i32 {
    (!x.is_nan() && !x.is_infinite()) as i32
}

#[no_mangle]
pub unsafe extern "C" fn finitef(x: f32) -> i32 {
    (!x.is_nan() && !x.is_infinite()) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __fpclassifyl(x: f64) -> i32 {
    __fpclassify(x)
}

#[no_mangle]
pub unsafe extern "C" fn __signbitl(x: f64) -> i32 {
    __signbit(x)
}

// ============================================================
// Bessel functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn j0(x: f64) -> f64 {
    libm::j0(x)
}

#[no_mangle]
pub unsafe extern "C" fn j0f(x: f32) -> f32 {
    libm::j0f(x)
}

#[no_mangle]
pub unsafe extern "C" fn j1(x: f64) -> f64 {
    libm::j1(x)
}

#[no_mangle]
pub unsafe extern "C" fn j1f(x: f32) -> f32 {
    libm::j1f(x)
}

#[no_mangle]
pub unsafe extern "C" fn jn(n: i32, x: f64) -> f64 {
    libm::jn(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn jnf(n: i32, x: f32) -> f32 {
    libm::jnf(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn y0(x: f64) -> f64 {
    y0_impl(x)
}

#[no_mangle]
pub unsafe extern "C" fn y0f(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f32::NEG_INFINITY; }
    libm::y0f(x)
}

#[no_mangle]
pub unsafe extern "C" fn y1(x: f64) -> f64 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalid(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f64::NEG_INFINITY; }
    libm::y1(x)
}

#[no_mangle]
pub unsafe extern "C" fn y1f(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f32::NEG_INFINITY; }
    libm::y1f(x)
}

#[no_mangle]
pub unsafe extern "C" fn yn(n: i32, x: f64) -> f64 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalid(x); }
    if x == 0.0 {
        feraiseexcept(FE_DIVBYZERO);
        if n >= 0 { return f64::NEG_INFINITY; }
    }
    libm::yn(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn ynf(n: i32, x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 {
        feraiseexcept(FE_DIVBYZERO);
        if n >= 0 { return f32::NEG_INFINITY; }
    }
    libm::ynf(n, x)
}

// ============================================================
// nan functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn nan(_tagp: *const core::ffi::c_char) -> f64 {
    f64::NAN
}

#[no_mangle]
pub unsafe extern "C" fn nanf(_tagp: *const core::ffi::c_char) -> f32 {
    f32::NAN
}

// ============================================================
// Missing math functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn drem(x: f64, y: f64) -> f64 {
    remainder(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn dremf(x: f32, y: f32) -> f32 {
    remainderf(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn pow10(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn pow10f(x: f32) -> f32 {
    libm::powf(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn pow10l(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn isless(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn lgamma_r(x: f64, signgamp: *mut i32) -> f64 {
    lgamma_r_impl(x, signgamp)
}

#[no_mangle]
pub unsafe extern "C" fn lgammaf_r(x: f32, signgamp: *mut i32) -> f32 {
    if x.is_nan() {
        if !signgamp.is_null() { *signgamp = 1; }
        return x;
    }
    if x == 0.0 {
        if !signgamp.is_null() { *signgamp = if x.is_sign_negative() { -1 } else { 1 }; }
        feraiseexcept(FE_DIVBYZERO);
        return f32::INFINITY;
    }
    let (r, sign) = libm::lgammaf_r(x);
    if !signgamp.is_null() { *signgamp = sign; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn lgammal_r(x: f64, signgamp: *mut i32) -> f64 {
    lgamma_r(x, signgamp)
}

// ============================================================
// Comparison macros
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn islessequal(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x <= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessequalf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x <= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessgreater(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y || x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessgreaterf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y || x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreater(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterequal(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x >= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterequalf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x >= y) as i32 }
}

